mod params;

use crate::{
    client::GrampsClient,
    config::Config,
    tools::{create, delete, get, merge, search, transaction, update},
};
use params::{
    CreateCitationInput, CreateEventInput, CreateFamilyInput, CreateMediaInput, CreateNoteInput,
    CreatePersonInput, CreatePlaceInput, CreateRepositoryInput, CreateSourceInput, CreateTagInput,
    HandleInput, HandlePairInput, MergeFamilyInput, MergeInput, MergePersonInput, QueryInput,
    UndoInput, UpdateInput,
};
use rmcp::{
    handler::server::{
        tool::{ToolCallContext, ToolRouter},
        wrapper::Parameters,
    },
    model::{
        CallToolRequestParams, CallToolResult, Content, Implementation, ListToolsResult,
        PaginatedRequestParams, ServerCapabilities, ServerInfo,
    },
    service::RequestContext,
    tool, tool_handler, tool_router, ErrorData as McpError, RoleServer, ServerHandler,
};

pub struct GrampsMcpServer {
    client: GrampsClient,
    tools: ToolRouter<Self>,
}

impl GrampsMcpServer {
    pub fn new(config: Config) -> Result<Self, reqwest::Error> {
        let http = reqwest::Client::builder()
            .user_agent(concat!("gramps-mcp-rs/", env!("CARGO_PKG_VERSION")))
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
    "delete_citation",
    "delete_event",
    "delete_family",
    "delete_media",
    "delete_note",
    "delete_person",
    "delete_place",
    "delete_repository",
    "delete_source",
    "delete_tag",
    "merge_citation",
    "merge_event",
    "merge_family",
    "merge_media",
    "merge_note",
    "merge_person",
    "merge_place",
    "merge_repository",
    "merge_source",
    "undo_transaction",
];

#[tool_router]
impl GrampsMcpServer {
    // ── Search ──────────────────────────────────────────────────────────────

    #[tool(description = "Search for people in the genealogy database")]
    async fn find_person(
        &self,
        Parameters(QueryInput { query }): Parameters<QueryInput>,
    ) -> Result<CallToolResult, McpError> {
        let items = search::find_person(&self.client, &query)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let text = serde_json::to_string_pretty(&items).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Search for families in the genealogy database")]
    async fn find_family(
        &self,
        Parameters(QueryInput { query }): Parameters<QueryInput>,
    ) -> Result<CallToolResult, McpError> {
        let items = search::find_family(&self.client, &query)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let text = serde_json::to_string_pretty(&items).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Search for events in the genealogy database")]
    async fn find_event(
        &self,
        Parameters(QueryInput { query }): Parameters<QueryInput>,
    ) -> Result<CallToolResult, McpError> {
        let items = search::find_event(&self.client, &query)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let text = serde_json::to_string_pretty(&items).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Search for places in the genealogy database")]
    async fn find_place(
        &self,
        Parameters(QueryInput { query }): Parameters<QueryInput>,
    ) -> Result<CallToolResult, McpError> {
        let items = search::find_place(&self.client, &query)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let text = serde_json::to_string_pretty(&items).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Search for notes in the genealogy database")]
    async fn find_note(
        &self,
        Parameters(QueryInput { query }): Parameters<QueryInput>,
    ) -> Result<CallToolResult, McpError> {
        let items = search::find_note(&self.client, &query)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let text = serde_json::to_string_pretty(&items).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Search for tags in the genealogy database")]
    async fn find_tag(
        &self,
        Parameters(QueryInput { query }): Parameters<QueryInput>,
    ) -> Result<CallToolResult, McpError> {
        let items = search::find_tag(&self.client, &query)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let text = serde_json::to_string_pretty(&items).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Search for citations in the genealogy database")]
    async fn find_citation(
        &self,
        Parameters(QueryInput { query }): Parameters<QueryInput>,
    ) -> Result<CallToolResult, McpError> {
        let items = search::find_citation(&self.client, &query)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let text = serde_json::to_string_pretty(&items).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Search for media objects in the genealogy database")]
    async fn find_media(
        &self,
        Parameters(QueryInput { query }): Parameters<QueryInput>,
    ) -> Result<CallToolResult, McpError> {
        let items = search::find_media(&self.client, &query)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let text = serde_json::to_string_pretty(&items).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Search for repositories in the genealogy database")]
    async fn find_repository(
        &self,
        Parameters(QueryInput { query }): Parameters<QueryInput>,
    ) -> Result<CallToolResult, McpError> {
        let items = search::find_repository(&self.client, &query)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let text = serde_json::to_string_pretty(&items).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Search for sources in the genealogy database")]
    async fn find_source(
        &self,
        Parameters(QueryInput { query }): Parameters<QueryInput>,
    ) -> Result<CallToolResult, McpError> {
        let items = search::find_source(&self.client, &query)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let text = serde_json::to_string_pretty(&items).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Full-text search across all genealogy object types")]
    async fn find_anything(
        &self,
        Parameters(QueryInput { query }): Parameters<QueryInput>,
    ) -> Result<CallToolResult, McpError> {
        let result = search::find_anything(&self.client, &query)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let text = serde_json::to_string_pretty(&result).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    // ── Get ─────────────────────────────────────────────────────────────────

    #[tool(description = "Get full details for a person by handle")]
    async fn get_person(
        &self,
        Parameters(HandleInput { handle }): Parameters<HandleInput>,
    ) -> Result<CallToolResult, McpError> {
        let person = get::get_person(&self.client, &handle)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let text = serde_json::to_string_pretty(&person).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Get full details for a family by handle")]
    async fn get_family(
        &self,
        Parameters(HandleInput { handle }): Parameters<HandleInput>,
    ) -> Result<CallToolResult, McpError> {
        let family = get::get_family(&self.client, &handle)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let text = serde_json::to_string_pretty(&family).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Get full details for an event by handle")]
    async fn get_event(
        &self,
        Parameters(HandleInput { handle }): Parameters<HandleInput>,
    ) -> Result<CallToolResult, McpError> {
        let item = get::get_event(&self.client, &handle)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let text = serde_json::to_string_pretty(&item).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Get full details for a place by handle")]
    async fn get_place(
        &self,
        Parameters(HandleInput { handle }): Parameters<HandleInput>,
    ) -> Result<CallToolResult, McpError> {
        let item = get::get_place(&self.client, &handle)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let text = serde_json::to_string_pretty(&item).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Get full details for a citation by handle")]
    async fn get_citation(
        &self,
        Parameters(HandleInput { handle }): Parameters<HandleInput>,
    ) -> Result<CallToolResult, McpError> {
        let item = get::get_citation(&self.client, &handle)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let text = serde_json::to_string_pretty(&item).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Get full details for a note by handle")]
    async fn get_note(
        &self,
        Parameters(HandleInput { handle }): Parameters<HandleInput>,
    ) -> Result<CallToolResult, McpError> {
        let item = get::get_note(&self.client, &handle)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let text = serde_json::to_string_pretty(&item).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Get full details for a media object by handle")]
    async fn get_media(
        &self,
        Parameters(HandleInput { handle }): Parameters<HandleInput>,
    ) -> Result<CallToolResult, McpError> {
        let item = get::get_media(&self.client, &handle)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let text = serde_json::to_string_pretty(&item).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Get full details for a repository by handle")]
    async fn get_repository(
        &self,
        Parameters(HandleInput { handle }): Parameters<HandleInput>,
    ) -> Result<CallToolResult, McpError> {
        let item = get::get_repository(&self.client, &handle)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let text = serde_json::to_string_pretty(&item).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Get full details for a tag by handle")]
    async fn get_tag(
        &self,
        Parameters(HandleInput { handle }): Parameters<HandleInput>,
    ) -> Result<CallToolResult, McpError> {
        let item = get::get_tag(&self.client, &handle)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let text = serde_json::to_string_pretty(&item).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Get full details for a source by handle")]
    async fn get_source(
        &self,
        Parameters(HandleInput { handle }): Parameters<HandleInput>,
    ) -> Result<CallToolResult, McpError> {
        let item = get::get_source(&self.client, &handle)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let text = serde_json::to_string_pretty(&item).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Get the most direct relationship path between two people")]
    async fn get_relations(
        &self,
        Parameters(HandlePairInput { handle1, handle2 }): Parameters<HandlePairInput>,
    ) -> Result<CallToolResult, McpError> {
        let item = get::get_relations(&self.client, &handle1, &handle2)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let text = serde_json::to_string_pretty(&item).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Get chronological event timeline for a person")]
    async fn get_person_timeline(
        &self,
        Parameters(HandleInput { handle }): Parameters<HandleInput>,
    ) -> Result<CallToolResult, McpError> {
        let item = get::get_person_timeline(&self.client, &handle)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let text = serde_json::to_string_pretty(&item).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Get chronological event timeline for a family")]
    async fn get_family_timeline(
        &self,
        Parameters(HandleInput { handle }): Parameters<HandleInput>,
    ) -> Result<CallToolResult, McpError> {
        let item = get::get_family_timeline(&self.client, &handle)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let text = serde_json::to_string_pretty(&item).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Get the time span between two events (e.g. birth and death)")]
    async fn get_event_span(
        &self,
        Parameters(HandlePairInput { handle1, handle2 }): Parameters<HandlePairInput>,
    ) -> Result<CallToolResult, McpError> {
        let item = get::get_event_span(&self.client, &handle1, &handle2)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let text = serde_json::to_string_pretty(&item).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(
        description = "List recent transactions (use transaction_id with undo_transaction to roll back)"
    )]
    async fn list_transactions(&self) -> Result<CallToolResult, McpError> {
        let result = transaction::list_transactions(&self.client)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let text = serde_json::to_string_pretty(&result).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Get tree-level statistics and metadata")]
    async fn get_tree_info(&self) -> Result<CallToolResult, McpError> {
        let info = get::get_tree_info(&self.client)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let text = serde_json::to_string_pretty(&info).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
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

        let handle = create::create_person(&self.client, req)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Created person with handle: {handle}"
        ))]))
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

        let handle = create::create_family(&self.client, req)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Created family with handle: {handle}"
        ))]))
    }

    #[tool(description = "Create a new event (birth, death, marriage, etc.)")]
    async fn create_event(
        &self,
        Parameters(CreateEventInput {
            event_type,
            description,
            date_text,
            place_handle,
        }): Parameters<CreateEventInput>,
    ) -> Result<CallToolResult, McpError> {
        use crate::models::{event::CreateEventRequest, GrampsDate};

        let date = date_text.map(|text| GrampsDate {
            text: Some(text),
            ..Default::default()
        });

        let req = CreateEventRequest {
            event_type: Some(serde_json::json!(event_type)),
            description,
            date,
            place: place_handle,
            ..Default::default()
        };

        let handle = create::create_event(&self.client, req)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Created event with handle: {handle}"
        ))]))
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

        let handle = create::create_place(&self.client, req)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Created place with handle: {handle}"
        ))]))
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

        let handle = create::create_source(&self.client, req)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Created source with handle: {handle}"
        ))]))
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
        let handle = create::create_tag(&self.client, &name, color.as_deref(), priority)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Created tag with handle: {handle}"
        ))]))
    }

    #[tool(description = "Create a new citation linking a source by handle")]
    async fn create_citation(
        &self,
        Parameters(CreateCitationInput {
            source_handle,
            page,
        }): Parameters<CreateCitationInput>,
    ) -> Result<CallToolResult, McpError> {
        let handle = create::create_citation(&self.client, &source_handle, page.as_deref())
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Created citation with handle: {handle}"
        ))]))
    }

    #[tool(description = "Create a new repository record")]
    async fn create_repository(
        &self,
        Parameters(CreateRepositoryInput { name, repo_type }): Parameters<CreateRepositoryInput>,
    ) -> Result<CallToolResult, McpError> {
        let handle = create::create_repository(&self.client, &name, repo_type.as_deref())
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Created repository with handle: {handle}"
        ))]))
    }

    #[tool(description = "Create a text note")]
    async fn create_note(
        &self,
        Parameters(CreateNoteInput { text, note_type }): Parameters<CreateNoteInput>,
    ) -> Result<CallToolResult, McpError> {
        let handle = create::create_note(&self.client, &text, note_type.as_deref())
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Created note with handle: {handle}"
        ))]))
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
        let handle = match (path.as_deref(), url.as_deref()) {
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
                return Err(McpError::invalid_params(
                    "Either path or url must be provided",
                    None,
                ))
            }
        }
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Created media with handle: {handle}"
        ))]))
    }

    // ── Update ──────────────────────────────────────────────────────────────

    #[tool(
        description = "Update an existing person. Pass the full object from get_person with modifications."
    )]
    async fn update_person(
        &self,
        Parameters(UpdateInput { handle, data }): Parameters<UpdateInput>,
    ) -> Result<CallToolResult, McpError> {
        let result = update::update_person(&self.client, &handle, &data)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let text = serde_json::to_string_pretty(&result).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(
        description = "Update an existing family. Pass the full object from get_family with modifications."
    )]
    async fn update_family(
        &self,
        Parameters(UpdateInput { handle, data }): Parameters<UpdateInput>,
    ) -> Result<CallToolResult, McpError> {
        let result = update::update_family(&self.client, &handle, &data)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let text = serde_json::to_string_pretty(&result).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(
        description = "Update an existing event. Pass the full object from get_event with modifications."
    )]
    async fn update_event(
        &self,
        Parameters(UpdateInput { handle, data }): Parameters<UpdateInput>,
    ) -> Result<CallToolResult, McpError> {
        let result = update::update_event(&self.client, &handle, &data)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let text = serde_json::to_string_pretty(&result).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(
        description = "Update an existing place. Pass the full object from get_place with modifications."
    )]
    async fn update_place(
        &self,
        Parameters(UpdateInput { handle, data }): Parameters<UpdateInput>,
    ) -> Result<CallToolResult, McpError> {
        let result = update::update_place(&self.client, &handle, &data)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let text = serde_json::to_string_pretty(&result).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(
        description = "Update an existing source. Pass the full object from get_source with modifications."
    )]
    async fn update_source(
        &self,
        Parameters(UpdateInput { handle, data }): Parameters<UpdateInput>,
    ) -> Result<CallToolResult, McpError> {
        let result = update::update_source(&self.client, &handle, &data)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let text = serde_json::to_string_pretty(&result).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(
        description = "Update an existing citation. Pass the full object from get_citation with modifications."
    )]
    async fn update_citation(
        &self,
        Parameters(UpdateInput { handle, data }): Parameters<UpdateInput>,
    ) -> Result<CallToolResult, McpError> {
        let result = update::update_citation(&self.client, &handle, &data)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let text = serde_json::to_string_pretty(&result).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(
        description = "Update an existing repository. Pass the full object from get_repository with modifications."
    )]
    async fn update_repository(
        &self,
        Parameters(UpdateInput { handle, data }): Parameters<UpdateInput>,
    ) -> Result<CallToolResult, McpError> {
        let result = update::update_repository(&self.client, &handle, &data)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let text = serde_json::to_string_pretty(&result).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(
        description = "Update an existing note. Pass the full object from get_note with modifications."
    )]
    async fn update_note(
        &self,
        Parameters(UpdateInput { handle, data }): Parameters<UpdateInput>,
    ) -> Result<CallToolResult, McpError> {
        let result = update::update_note(&self.client, &handle, &data)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let text = serde_json::to_string_pretty(&result).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(
        description = "Update an existing tag. Pass the full object from get_tag with modifications."
    )]
    async fn update_tag(
        &self,
        Parameters(UpdateInput { handle, data }): Parameters<UpdateInput>,
    ) -> Result<CallToolResult, McpError> {
        let result = update::update_tag(&self.client, &handle, &data)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let text = serde_json::to_string_pretty(&result).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(
        description = "Update an existing media object. Pass the full object from get_media with modifications."
    )]
    async fn update_media(
        &self,
        Parameters(UpdateInput { handle, data }): Parameters<UpdateInput>,
    ) -> Result<CallToolResult, McpError> {
        let result = update::update_media(&self.client, &handle, &data)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let text = serde_json::to_string_pretty(&result).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    // ── Delete ──────────────────────────────────────────────────────────────

    #[tool(description = "Delete a person by handle")]
    async fn delete_person(
        &self,
        Parameters(HandleInput { handle }): Parameters<HandleInput>,
    ) -> Result<CallToolResult, McpError> {
        delete::delete_person(&self.client, &handle)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Deleted person {handle}"
        ))]))
    }

    #[tool(description = "Delete a family by handle")]
    async fn delete_family(
        &self,
        Parameters(HandleInput { handle }): Parameters<HandleInput>,
    ) -> Result<CallToolResult, McpError> {
        delete::delete_family(&self.client, &handle)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Deleted family {handle}"
        ))]))
    }

    #[tool(description = "Delete an event by handle")]
    async fn delete_event(
        &self,
        Parameters(HandleInput { handle }): Parameters<HandleInput>,
    ) -> Result<CallToolResult, McpError> {
        delete::delete_event(&self.client, &handle)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Deleted event {handle}"
        ))]))
    }

    #[tool(description = "Delete a place by handle")]
    async fn delete_place(
        &self,
        Parameters(HandleInput { handle }): Parameters<HandleInput>,
    ) -> Result<CallToolResult, McpError> {
        delete::delete_place(&self.client, &handle)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Deleted place {handle}"
        ))]))
    }

    #[tool(description = "Delete a source by handle")]
    async fn delete_source(
        &self,
        Parameters(HandleInput { handle }): Parameters<HandleInput>,
    ) -> Result<CallToolResult, McpError> {
        delete::delete_source(&self.client, &handle)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Deleted source {handle}"
        ))]))
    }

    #[tool(description = "Delete a citation by handle")]
    async fn delete_citation(
        &self,
        Parameters(HandleInput { handle }): Parameters<HandleInput>,
    ) -> Result<CallToolResult, McpError> {
        delete::delete_citation(&self.client, &handle)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Deleted citation {handle}"
        ))]))
    }

    #[tool(description = "Delete a repository by handle")]
    async fn delete_repository(
        &self,
        Parameters(HandleInput { handle }): Parameters<HandleInput>,
    ) -> Result<CallToolResult, McpError> {
        delete::delete_repository(&self.client, &handle)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Deleted repository {handle}"
        ))]))
    }

    #[tool(description = "Delete a note by handle")]
    async fn delete_note(
        &self,
        Parameters(HandleInput { handle }): Parameters<HandleInput>,
    ) -> Result<CallToolResult, McpError> {
        delete::delete_note(&self.client, &handle)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Deleted note {handle}"
        ))]))
    }

    #[tool(description = "Delete a tag by handle")]
    async fn delete_tag(
        &self,
        Parameters(HandleInput { handle }): Parameters<HandleInput>,
    ) -> Result<CallToolResult, McpError> {
        delete::delete_tag(&self.client, &handle)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Deleted tag {handle}"
        ))]))
    }

    #[tool(description = "Delete a media object by handle")]
    async fn delete_media(
        &self,
        Parameters(HandleInput { handle }): Parameters<HandleInput>,
    ) -> Result<CallToolResult, McpError> {
        delete::delete_media(&self.client, &handle)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Deleted media {handle}"
        ))]))
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
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Merged person {duplicate_handle} into {survivor_handle}"
        ))]))
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
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Merged family {duplicate_handle} into {survivor_handle}"
        ))]))
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
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Merged citation {duplicate_handle} into {survivor_handle}"
        ))]))
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
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Merged event {duplicate_handle} into {survivor_handle}"
        ))]))
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
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Merged media {duplicate_handle} into {survivor_handle}"
        ))]))
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
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Merged note {duplicate_handle} into {survivor_handle}"
        ))]))
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
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Merged place {duplicate_handle} into {survivor_handle}"
        ))]))
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
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Merged repository {duplicate_handle} into {survivor_handle}"
        ))]))
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
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Merged source {duplicate_handle} into {survivor_handle}"
        ))]))
    }

    #[tool(description = "Undo a transaction by its ID (obtain ID from list_transactions)")]
    async fn undo_transaction(
        &self,
        Parameters(UndoInput { transaction_id }): Parameters<UndoInput>,
    ) -> Result<CallToolResult, McpError> {
        transaction::undo_transaction(&self.client, transaction_id)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Transaction {transaction_id} undone"
        ))]))
    }
}

#[tool_handler]
impl ServerHandler for GrampsMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions("MCP server for querying and updating genealogy data in Gramps Web.")
            .with_server_info(Implementation::new("gramps-mcp", env!("CARGO_PKG_VERSION")))
    }

    // WORKAROUND: Claude Desktop does not recognise JSON Schema draft 2020-12
    // (produced by schemars 1.x / rmcp 1.7) and silently hides all tools in
    // the chat UI when it encounters it.  We patch every inputSchema on the fly:
    //   1. Replace the $schema URI with the draft-07 declaration.
    //   2. Replace boolean sub-schemas (draft-06+: `true` / `false`) with their
    //      object equivalents (`{}` / `{"not":{}}`), which draft-07 requires.
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
        self.tools
            .call(ToolCallContext::new(self, request, context))
            .await
    }
}

// See the WORKAROUND comment on list_tools above.
fn fix_schema(v: &mut serde_json::Value) {
    match v {
        serde_json::Value::Bool(true) => *v = serde_json::json!({}),
        serde_json::Value::Bool(false) => *v = serde_json::json!({"not": {}}),
        serde_json::Value::Object(m) => m.values_mut().for_each(fix_schema),
        serde_json::Value::Array(arr) => arr.iter_mut().for_each(fix_schema),
        _ => {}
    }
}
