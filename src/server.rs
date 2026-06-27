// Copyright 2026 Alexey Chernyshov
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

mod params;

use crate::{
    client::GrampsClient,
    config::Config,
    tools::{create, delete, get, merge, search, update},
};
use params::{
    CreateCitationInput, CreateEventInput, CreateFamilyInput, CreateMediaInput, CreateNoteInput,
    CreatePersonInput, CreatePlaceInput, CreateRepositoryInput, CreateSourceInput, CreateTagInput,
    DeleteObjectInput, GetObjectInput, HandleInput, HandlePairInput, MergeFamilyInput, MergeInput,
    MergePersonInput, SearchInput, UpdateInput,
};
use rmcp::{
    handler::server::{
        tool::{ToolCallContext, ToolRouter},
        wrapper::Parameters,
    },
    model::{
        CallToolRequestParams, CallToolResult, Content, ErrorCode, Implementation,
        ListResourcesResult, ListToolsResult, PaginatedRequestParams, RawResource,
        ReadResourceRequestParams, ReadResourceResult, ResourceContents, ServerCapabilities,
        ServerInfo,
    },
    service::RequestContext,
    tool, tool_handler, tool_router, ErrorData as McpError, RoleServer, ServerHandler,
};

#[derive(Clone)]
pub struct GrampsMcpServer {
    client: GrampsClient,
    tools: ToolRouter<Self>,
}

impl GrampsMcpServer {
    pub fn new(config: Config) -> Result<Self, reqwest::Error> {
        let http = reqwest::Client::builder()
            .user_agent(concat!("gramps-web-mcp-rs/", env!("CARGO_PKG_VERSION")))
            .build()?;
        let mut tools = Self::tool_router();
        if config.gramps_readonly {
            for name in WRITE_TOOLS {
                tools.disable_route(*name);
            }
        }
        Ok(Self {
            client: GrampsClient::new(config, http),
            tools,
        })
    }
}

const WRITE_TOOLS: &[&str] = &[
    "create_citation",
    "create_media",
    "create_tag",
    "create_event",
    "create_family",
    "create_note",
    "create_person",
    "create_place",
    "create_repository",
    "create_source",
    "update_citation",
    "update_event",
    "update_family",
    "update_media",
    "update_note",
    "update_person",
    "update_place",
    "update_repository",
    "update_source",
    "update_tag",
    "delete_object",
    "merge_citation",
    "merge_event",
    "merge_family",
    "merge_media",
    "merge_note",
    "merge_person",
    "merge_place",
    "merge_repository",
    "merge_source",
];

fn ok_json(v: serde_json::Value) -> Result<CallToolResult, McpError> {
    let text = serde_json::to_string_pretty(&v).unwrap_or_default();
    Ok(CallToolResult::success(vec![Content::text(text)]))
}

fn api_err(e: impl std::fmt::Display) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::error(vec![Content::text(e.to_string())]))
}

fn require_object(data: &serde_json::Value) -> Option<Result<CallToolResult, McpError>> {
    if data.is_object() {
        None
    } else {
        Some(Ok(CallToolResult::error(vec![Content::text(
            "`data` must be a JSON object — pass the full object returned by get_object, not a string",
        )])))
    }
}

#[tool_router]
impl GrampsMcpServer {
    // ── Search ──────────────────────────────────────────────────────────────

    #[tool(description = "\
Full-text search across the genealogy database. \
Set object_type to narrow results to a specific type, or omit to search across all types. \
Use page/pagesize to paginate large result sets (default page=1, pagesize=20).")]
    async fn search(
        &self,
        Parameters(SearchInput {
            query,
            object_type,
            page,
            pagesize,
        }): Parameters<SearchInput>,
    ) -> Result<CallToolResult, McpError> {
        search::search(
            &self.client,
            &query,
            object_type.map(|t| t.as_str()),
            page,
            pagesize,
        )
        .await
        .map_or_else(api_err, ok_json)
    }

    // ── Get ─────────────────────────────────────────────────────────────────

    #[tool(
        description = "Get the OQL (Object Query Language) reference: operators, \
helper methods, and query examples by object type. \
Call this before writing an oql filter."
    )]
    async fn get_oql_reference(&self) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![Content::text(OQL_REFERENCE)]))
    }

    #[tool(description = "\
Get genealogy objects. \
`object_type` is always required (person, family, event, place, note, citation, source, media, repository, tag). \
Provide `handle` for a single record, or `gramps_id` / `oql` / `page` / `pagesize` to browse a collection. \
Use `oql` for structured filtering (call get_oql_reference for syntax). \
For full-text search use the `search` tool instead.")]
    async fn get_object(
        &self,
        Parameters(GetObjectInput {
            object_type,
            handle,
            gramps_id,
            oql,
            page,
            pagesize,
        }): Parameters<GetObjectInput>,
    ) -> Result<CallToolResult, McpError> {
        let result = if let Some(h) = handle {
            get::get_object_by_handle(&self.client, object_type.as_endpoint(), &h).await
        } else if gramps_id.is_some() || oql.is_some() || page.is_some() || pagesize.is_some() {
            get::get_object_collection(
                &self.client,
                object_type.as_endpoint(),
                gramps_id.as_deref(),
                oql.as_deref(),
                page,
                pagesize,
            )
            .await
        } else {
            return Ok(CallToolResult::error(vec![Content::text(
                "Provide `handle` for a single object, \
                 or `gramps_id` / `oql` / `page` / `pagesize` to browse a collection.",
            )]));
        };
        result.map_or_else(api_err, ok_json)
    }

    #[tool(description = "Get the most direct relationship path between two people")]
    async fn get_relations(
        &self,
        Parameters(HandlePairInput { handle1, handle2 }): Parameters<HandlePairInput>,
    ) -> Result<CallToolResult, McpError> {
        get::get_relations(&self.client, &handle1, &handle2)
            .await
            .map_or_else(api_err, ok_json)
    }

    #[tool(description = "Get chronological event timeline for a person")]
    async fn get_person_timeline(
        &self,
        Parameters(HandleInput { handle }): Parameters<HandleInput>,
    ) -> Result<CallToolResult, McpError> {
        get::get_person_timeline(&self.client, &handle)
            .await
            .map_or_else(api_err, ok_json)
    }

    #[tool(description = "Get chronological event timeline for a family")]
    async fn get_family_timeline(
        &self,
        Parameters(HandleInput { handle }): Parameters<HandleInput>,
    ) -> Result<CallToolResult, McpError> {
        get::get_family_timeline(&self.client, &handle)
            .await
            .map_or_else(api_err, ok_json)
    }

    #[tool(description = "Get the time span between two events (e.g. birth and death)")]
    async fn get_event_span(
        &self,
        Parameters(HandlePairInput { handle1, handle2 }): Parameters<HandlePairInput>,
    ) -> Result<CallToolResult, McpError> {
        get::get_event_span(&self.client, &handle1, &handle2)
            .await
            .map_or_else(api_err, ok_json)
    }

    #[tool(description = "Get tree-level statistics and metadata")]
    async fn get_tree_info(&self) -> Result<CallToolResult, McpError> {
        get::get_tree_info(&self.client)
            .await
            .map_or_else(api_err, ok_json)
    }

    // ── Create ──────────────────────────────────────────────────────────────

    #[tool(description = "Create a new person. Provide first_name and/or surname.")]
    async fn create_person(
        &self,
        Parameters(CreatePersonInput {
            first_name,
            surname,
            gender,
        }): Parameters<CreatePersonInput>,
    ) -> Result<CallToolResult, McpError> {
        use crate::models::person::{CreatePersonRequest, PersonName, Surname};

        let surname_list = surname
            .map(|s| {
                vec![Surname {
                    surname: Some(s),
                    ..Default::default()
                }]
            })
            .unwrap_or_default();

        let primary_name = Some(PersonName {
            first_name,
            surname_list,
            name_type: Some("Birth Name".into()),
            ..Default::default()
        });

        let req = CreatePersonRequest {
            primary_name,
            gender,
            ..Default::default()
        };

        create::create_person(&self.client, req)
            .await
            .map_or_else(api_err, |handle| {
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Created person with handle: {handle}"
                ))]))
            })
    }

    #[tool(description = "Create a new family linking father and/or mother by their handles")]
    async fn create_family(
        &self,
        Parameters(CreateFamilyInput {
            father_handle,
            mother_handle,
        }): Parameters<CreateFamilyInput>,
    ) -> Result<CallToolResult, McpError> {
        use crate::models::family::CreateFamilyRequest;

        let req = CreateFamilyRequest {
            father_handle,
            mother_handle,
            ..Default::default()
        };

        create::create_family(&self.client, req)
            .await
            .map_or_else(api_err, |handle| {
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Created family with handle: {handle}"
                ))]))
            })
    }

    #[tool(description = "Create a new event (birth, death, marriage, etc.)")]
    async fn create_event(
        &self,
        Parameters(CreateEventInput {
            event_type,
            description,
            date,
            date_text,
            place_handle,
        }): Parameters<CreateEventInput>,
    ) -> Result<CallToolResult, McpError> {
        use crate::models::{event::CreateEventRequest, GrampsDate};

        let date = if let Some([d, m, y]) = date {
            Some(GrampsDate {
                dateval: Some(serde_json::json!([d, m, y, false])),
                ..Default::default()
            })
        } else {
            date_text.map(|text| GrampsDate {
                text: Some(text),
                ..Default::default()
            })
        };

        let req = CreateEventRequest {
            event_type: Some(serde_json::json!(event_type)),
            description,
            date,
            place: place_handle,
            ..Default::default()
        };

        create::create_event(&self.client, req)
            .await
            .map_or_else(api_err, |handle| {
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Created event with handle: {handle}"
                ))]))
            })
    }

    #[tool(description = "Create a new place record")]
    async fn create_place(
        &self,
        Parameters(CreatePlaceInput { title, place_type }): Parameters<CreatePlaceInput>,
    ) -> Result<CallToolResult, McpError> {
        use crate::models::place::{CreatePlaceRequest, PlaceName};

        let req = CreatePlaceRequest {
            title: Some(title.clone()),
            name: Some(PlaceName {
                value: Some(title),
                ..Default::default()
            }),
            place_type: place_type.map(|t| serde_json::json!(t)),
            ..Default::default()
        };

        create::create_place(&self.client, req)
            .await
            .map_or_else(api_err, |handle| {
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Created place with handle: {handle}"
                ))]))
            })
    }

    #[tool(description = "Create a new source record")]
    async fn create_source(
        &self,
        Parameters(CreateSourceInput {
            title,
            author,
            pubinfo,
        }): Parameters<CreateSourceInput>,
    ) -> Result<CallToolResult, McpError> {
        use crate::models::source::CreateSourceRequest;

        let req = CreateSourceRequest {
            title: Some(title),
            author,
            pubinfo,
            ..Default::default()
        };

        create::create_source(&self.client, req)
            .await
            .map_or_else(api_err, |handle| {
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Created source with handle: {handle}"
                ))]))
            })
    }

    #[tool(description = "Create a new tag with a name, optional color (hex) and priority")]
    async fn create_tag(
        &self,
        Parameters(CreateTagInput {
            name,
            color,
            priority,
        }): Parameters<CreateTagInput>,
    ) -> Result<CallToolResult, McpError> {
        create::create_tag(&self.client, &name, color.as_deref(), priority)
            .await
            .map_or_else(api_err, |handle| {
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Created tag with handle: {handle}"
                ))]))
            })
    }

    #[tool(description = "Create a new citation linking a source by handle")]
    async fn create_citation(
        &self,
        Parameters(CreateCitationInput {
            source_handle,
            page,
        }): Parameters<CreateCitationInput>,
    ) -> Result<CallToolResult, McpError> {
        create::create_citation(&self.client, &source_handle, page.as_deref())
            .await
            .map_or_else(api_err, |handle| {
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Created citation with handle: {handle}"
                ))]))
            })
    }

    #[tool(description = "Create a new repository record")]
    async fn create_repository(
        &self,
        Parameters(CreateRepositoryInput { name, repo_type }): Parameters<CreateRepositoryInput>,
    ) -> Result<CallToolResult, McpError> {
        create::create_repository(&self.client, &name, repo_type.as_deref())
            .await
            .map_or_else(api_err, |handle| {
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Created repository with handle: {handle}"
                ))]))
            })
    }

    #[tool(description = "Create a text note")]
    async fn create_note(
        &self,
        Parameters(CreateNoteInput { text, note_type }): Parameters<CreateNoteInput>,
    ) -> Result<CallToolResult, McpError> {
        create::create_note(&self.client, &text, note_type.as_deref())
            .await
            .map_or_else(api_err, |handle| {
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Created note with handle: {handle}"
                ))]))
            })
    }

    #[tool(
        description = "Create a media record from a file already on the Gramps server (provide path) or by downloading from a URL (provide url)."
    )]
    async fn create_media(
        &self,
        Parameters(CreateMediaInput {
            path,
            url,
            description,
            mime,
        }): Parameters<CreateMediaInput>,
    ) -> Result<CallToolResult, McpError> {
        match (path.as_deref(), url.as_deref()) {
            (_, Some(url)) => {
                create::create_media_from_url(
                    &self.client,
                    url,
                    description.as_deref(),
                    mime.as_deref(),
                )
                .await
            }
            (Some(path), None) => {
                create::create_media_from_path(
                    &self.client,
                    path,
                    description.as_deref(),
                    mime.as_deref(),
                )
                .await
            }
            (None, None) => {
                return Ok(CallToolResult::error(vec![Content::text(
                    "Either path or url must be provided",
                )]))
            }
        }
        .map_or_else(api_err, |handle| {
            Ok(CallToolResult::success(vec![Content::text(format!(
                "Created media with handle: {handle}"
            ))]))
        })
    }

    // ── Update ──────────────────────────────────────────────────────────────

    #[tool(
        description = "Update an existing person. Pass the full object from get_person with modifications."
    )]
    async fn update_person(
        &self,
        Parameters(UpdateInput { handle, data }): Parameters<UpdateInput>,
    ) -> Result<CallToolResult, McpError> {
        if let Some(err) = require_object(&data) {
            return err;
        }
        update::update_person(&self.client, &handle, &data)
            .await
            .map_or_else(api_err, ok_json)
    }

    #[tool(
        description = "Update an existing family. Pass the full object from get_family with modifications."
    )]
    async fn update_family(
        &self,
        Parameters(UpdateInput { handle, data }): Parameters<UpdateInput>,
    ) -> Result<CallToolResult, McpError> {
        if let Some(err) = require_object(&data) {
            return err;
        }
        update::update_family(&self.client, &handle, &data)
            .await
            .map_or_else(api_err, ok_json)
    }

    #[tool(
        description = "Update an existing event. Pass the full object from get_event with modifications."
    )]
    async fn update_event(
        &self,
        Parameters(UpdateInput { handle, data }): Parameters<UpdateInput>,
    ) -> Result<CallToolResult, McpError> {
        if let Some(err) = require_object(&data) {
            return err;
        }
        update::update_event(&self.client, &handle, &data)
            .await
            .map_or_else(api_err, ok_json)
    }

    #[tool(
        description = "Update an existing place. Pass the full object from get_place with modifications."
    )]
    async fn update_place(
        &self,
        Parameters(UpdateInput { handle, data }): Parameters<UpdateInput>,
    ) -> Result<CallToolResult, McpError> {
        if let Some(err) = require_object(&data) {
            return err;
        }
        update::update_place(&self.client, &handle, &data)
            .await
            .map_or_else(api_err, ok_json)
    }

    #[tool(
        description = "Update an existing source. Pass the full object from get_source with modifications."
    )]
    async fn update_source(
        &self,
        Parameters(UpdateInput { handle, data }): Parameters<UpdateInput>,
    ) -> Result<CallToolResult, McpError> {
        if let Some(err) = require_object(&data) {
            return err;
        }
        update::update_source(&self.client, &handle, &data)
            .await
            .map_or_else(api_err, ok_json)
    }

    #[tool(
        description = "Update an existing citation. Pass the full object from get_citation with modifications."
    )]
    async fn update_citation(
        &self,
        Parameters(UpdateInput { handle, data }): Parameters<UpdateInput>,
    ) -> Result<CallToolResult, McpError> {
        if let Some(err) = require_object(&data) {
            return err;
        }
        update::update_citation(&self.client, &handle, &data)
            .await
            .map_or_else(api_err, ok_json)
    }

    #[tool(
        description = "Update an existing repository. Pass the full object from get_repository with modifications."
    )]
    async fn update_repository(
        &self,
        Parameters(UpdateInput { handle, data }): Parameters<UpdateInput>,
    ) -> Result<CallToolResult, McpError> {
        if let Some(err) = require_object(&data) {
            return err;
        }
        update::update_repository(&self.client, &handle, &data)
            .await
            .map_or_else(api_err, ok_json)
    }

    #[tool(
        description = "Update an existing note. Pass the full object from get_note with modifications."
    )]
    async fn update_note(
        &self,
        Parameters(UpdateInput { handle, data }): Parameters<UpdateInput>,
    ) -> Result<CallToolResult, McpError> {
        if let Some(err) = require_object(&data) {
            return err;
        }
        update::update_note(&self.client, &handle, &data)
            .await
            .map_or_else(api_err, ok_json)
    }

    #[tool(
        description = "Update an existing tag. Pass the full object from get_tag with modifications."
    )]
    async fn update_tag(
        &self,
        Parameters(UpdateInput { handle, data }): Parameters<UpdateInput>,
    ) -> Result<CallToolResult, McpError> {
        if let Some(err) = require_object(&data) {
            return err;
        }
        update::update_tag(&self.client, &handle, &data)
            .await
            .map_or_else(api_err, ok_json)
    }

    #[tool(
        description = "Update an existing media object. Pass the full object from get_media with modifications."
    )]
    async fn update_media(
        &self,
        Parameters(UpdateInput { handle, data }): Parameters<UpdateInput>,
    ) -> Result<CallToolResult, McpError> {
        if let Some(err) = require_object(&data) {
            return err;
        }
        update::update_media(&self.client, &handle, &data)
            .await
            .map_or_else(api_err, ok_json)
    }

    // ── Delete ──────────────────────────────────────────────────────────────

    #[tool(description = "Delete an object by handle")]
    async fn delete_object(
        &self,
        Parameters(DeleteObjectInput {
            object_type,
            handle,
        }): Parameters<DeleteObjectInput>,
    ) -> Result<CallToolResult, McpError> {
        delete::delete_object(&self.client, object_type.as_endpoint(), &handle)
            .await
            .map_or_else(api_err, |_| {
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Deleted {} {handle}",
                    object_type.as_str()
                ))]))
            })
    }

    // ── Merge ───────────────────────────────────────────────────────────────

    #[tool(
        description = "Merge two people: survivor_handle is kept, duplicate_handle is deleted. Set family_merger=true (default) to also merge their families."
    )]
    async fn merge_person(
        &self,
        Parameters(MergePersonInput {
            survivor_handle,
            duplicate_handle,
            family_merger,
        }): Parameters<MergePersonInput>,
    ) -> Result<CallToolResult, McpError> {
        merge::merge_person(
            &self.client,
            &survivor_handle,
            &duplicate_handle,
            family_merger.unwrap_or(true),
        )
        .await
        .map_or_else(api_err, |_| {
            Ok(CallToolResult::success(vec![Content::text(format!(
                "Merged person {duplicate_handle} into {survivor_handle}"
            ))]))
        })
    }

    #[tool(
        description = "Merge two families: survivor_handle is kept, duplicate_handle is deleted. Optionally specify which father/mother handle to keep."
    )]
    async fn merge_family(
        &self,
        Parameters(MergeFamilyInput {
            survivor_handle,
            duplicate_handle,
            phoenix_father_handle,
            phoenix_mother_handle,
        }): Parameters<MergeFamilyInput>,
    ) -> Result<CallToolResult, McpError> {
        merge::merge_family(
            &self.client,
            &survivor_handle,
            &duplicate_handle,
            phoenix_father_handle.as_deref(),
            phoenix_mother_handle.as_deref(),
        )
        .await
        .map_or_else(api_err, |_| {
            Ok(CallToolResult::success(vec![Content::text(format!(
                "Merged family {duplicate_handle} into {survivor_handle}"
            ))]))
        })
    }

    #[tool(
        description = "Merge two citations: survivor_handle is kept, duplicate_handle is deleted"
    )]
    async fn merge_citation(
        &self,
        Parameters(MergeInput {
            survivor_handle,
            duplicate_handle,
        }): Parameters<MergeInput>,
    ) -> Result<CallToolResult, McpError> {
        merge::merge_citation(&self.client, &survivor_handle, &duplicate_handle)
            .await
            .map_or_else(api_err, |_| {
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Merged citation {duplicate_handle} into {survivor_handle}"
                ))]))
            })
    }

    #[tool(description = "Merge two events: survivor_handle is kept, duplicate_handle is deleted")]
    async fn merge_event(
        &self,
        Parameters(MergeInput {
            survivor_handle,
            duplicate_handle,
        }): Parameters<MergeInput>,
    ) -> Result<CallToolResult, McpError> {
        merge::merge_event(&self.client, &survivor_handle, &duplicate_handle)
            .await
            .map_or_else(api_err, |_| {
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Merged event {duplicate_handle} into {survivor_handle}"
                ))]))
            })
    }

    #[tool(
        description = "Merge two media objects: survivor_handle is kept, duplicate_handle is deleted"
    )]
    async fn merge_media(
        &self,
        Parameters(MergeInput {
            survivor_handle,
            duplicate_handle,
        }): Parameters<MergeInput>,
    ) -> Result<CallToolResult, McpError> {
        merge::merge_media(&self.client, &survivor_handle, &duplicate_handle)
            .await
            .map_or_else(api_err, |_| {
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Merged media {duplicate_handle} into {survivor_handle}"
                ))]))
            })
    }

    #[tool(description = "Merge two notes: survivor_handle is kept, duplicate_handle is deleted")]
    async fn merge_note(
        &self,
        Parameters(MergeInput {
            survivor_handle,
            duplicate_handle,
        }): Parameters<MergeInput>,
    ) -> Result<CallToolResult, McpError> {
        merge::merge_note(&self.client, &survivor_handle, &duplicate_handle)
            .await
            .map_or_else(api_err, |_| {
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Merged note {duplicate_handle} into {survivor_handle}"
                ))]))
            })
    }

    #[tool(description = "Merge two places: survivor_handle is kept, duplicate_handle is deleted")]
    async fn merge_place(
        &self,
        Parameters(MergeInput {
            survivor_handle,
            duplicate_handle,
        }): Parameters<MergeInput>,
    ) -> Result<CallToolResult, McpError> {
        merge::merge_place(&self.client, &survivor_handle, &duplicate_handle)
            .await
            .map_or_else(api_err, |_| {
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Merged place {duplicate_handle} into {survivor_handle}"
                ))]))
            })
    }

    #[tool(
        description = "Merge two repositories: survivor_handle is kept, duplicate_handle is deleted"
    )]
    async fn merge_repository(
        &self,
        Parameters(MergeInput {
            survivor_handle,
            duplicate_handle,
        }): Parameters<MergeInput>,
    ) -> Result<CallToolResult, McpError> {
        merge::merge_repository(&self.client, &survivor_handle, &duplicate_handle)
            .await
            .map_or_else(api_err, |_| {
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Merged repository {duplicate_handle} into {survivor_handle}"
                ))]))
            })
    }

    #[tool(description = "Merge two sources: survivor_handle is kept, duplicate_handle is deleted")]
    async fn merge_source(
        &self,
        Parameters(MergeInput {
            survivor_handle,
            duplicate_handle,
        }): Parameters<MergeInput>,
    ) -> Result<CallToolResult, McpError> {
        merge::merge_source(&self.client, &survivor_handle, &duplicate_handle)
            .await
            .map_or_else(api_err, |_| {
                Ok(CallToolResult::success(vec![Content::text(format!(
                    "Merged source {duplicate_handle} into {survivor_handle}"
                ))]))
            })
    }
}

#[tool_handler]
impl ServerHandler for GrampsMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        )
        .with_instructions("MCP server for accessing genealogy data via the Gramps Web API.")
        .with_server_info(Implementation::new(
            "gramps-web-mcp",
            env!("CARGO_PKG_VERSION"),
        ))
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        let resource = RawResource::new("gramps://oql-reference", "oql-reference")
            .with_title("OQL Reference")
            .with_description("Object Query Language syntax, operators and helper method reference")
            .with_mime_type("text/markdown");
        Ok(ListResourcesResult {
            resources: vec![rmcp::model::Annotated::new(resource, None)],
            meta: None,
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        if request.uri == "gramps://oql-reference" {
            return Ok(ReadResourceResult::new(vec![ResourceContents::text(
                OQL_REFERENCE,
                "gramps://oql-reference",
            )]));
        }
        Err(McpError::invalid_params(
            format!("Unknown resource: {}", request.uri),
            None,
        ))
    }

    // WORKAROUND: Claude Desktop does not recognise JSON Schema draft 2020-12
    // (produced by schemars 1.x / rmcp 1.7) and silently hides all tools in
    // the chat UI when it encounters it.  We patch every inputSchema on the fly:
    //   1. Replace the $schema URI with the draft-07 declaration.
    //   2. Replace boolean sub-schemas (draft-06+: `true` / `false`) with their
    //      object equivalents (`{}` / `{"not":{}}`), which draft-07 requires.
    //   3. Rename "$defs" to "definitions" and update "$ref" paths accordingly.
    //      draft-07 uses "definitions"; "$defs" is 2019-09+. Without this,
    //      $ref resolution fails silently and enum constraints are lost.
    //
    // Track: https://github.com/modelcontextprotocol/rust-sdk/issues/326
    // Remove this override (and fix_schema below) once Claude Desktop supports
    // draft 2020-12, or once rmcp adds a built-in compatibility shim.
    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let tools = self
            .tools
            .list_all()
            .into_iter()
            .map(|mut tool| {
                let schema = std::sync::Arc::make_mut(&mut tool.input_schema);
                schema.insert(
                    "$schema".into(),
                    "http://json-schema.org/draft-07/schema#".into(),
                );
                if let Some(defs) = schema.remove("$defs") {
                    schema.insert("definitions".into(), defs);
                }
                schema.values_mut().for_each(fix_schema);
                tool
            })
            .collect();
        Ok(ListToolsResult {
            tools,
            meta: None,
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .tools
            .call(ToolCallContext::new(self, request, context))
            .await
        {
            Err(e) if e.code == ErrorCode::INVALID_PARAMS => {
                Ok(CallToolResult::error(vec![Content::text(format!(
                    "Invalid parameters: {}",
                    e.message
                ))]))
            }
            other => other,
        }
    }
}

const OQL_REFERENCE: &str = include_str!("resources/oql_reference.md");

// See the WORKAROUND comment on list_tools above.
fn fix_schema(v: &mut serde_json::Value) {
    match v {
        serde_json::Value::Bool(true) => *v = serde_json::json!({}),
        serde_json::Value::Bool(false) => *v = serde_json::json!({"not": {}}),
        serde_json::Value::String(s) if s.starts_with("#/$defs/") => {
            *s = s.replacen("#/$defs/", "#/definitions/", 1);
        }
        serde_json::Value::Object(m) => m.values_mut().for_each(fix_schema),
        serde_json::Value::Array(arr) => arr.iter_mut().for_each(fix_schema),
        _ => {}
    }
}
