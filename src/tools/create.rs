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

use crate::{
    client::{Error as ClientError, GrampsClient, Result},
    models::{
        event::CreateEventRequest, family::CreateFamilyRequest, person::CreatePersonRequest,
        place::CreatePlaceRequest, source::CreateSourceRequest, Handle,
    },
    tools::get,
};

pub async fn create_person(client: &GrampsClient, req: CreatePersonRequest) -> Result<Handle> {
    let resp: serde_json::Value = client.post("/api/people/", &req).await?;
    extract_handle(resp)
}

pub async fn create_family(client: &GrampsClient, req: CreateFamilyRequest) -> Result<Handle> {
    let resp: serde_json::Value = client.post("/api/families/", &req).await?;
    extract_handle(resp)
}

pub async fn create_event(client: &GrampsClient, req: CreateEventRequest) -> Result<Handle> {
    let resp: serde_json::Value = client.post("/api/events/", &req).await?;
    extract_handle(resp)
}

pub async fn create_place(client: &GrampsClient, req: CreatePlaceRequest) -> Result<Handle> {
    let resp: serde_json::Value = client.post("/api/places/", &req).await?;
    extract_handle(resp)
}

pub async fn create_source(client: &GrampsClient, req: CreateSourceRequest) -> Result<Handle> {
    let resp: serde_json::Value = client.post("/api/sources/", &req).await?;
    extract_handle(resp)
}

pub async fn create_note(
    client: &GrampsClient,
    text: &str,
    note_type: Option<&str>,
) -> Result<Handle> {
    let body = serde_json::json!({
        "text": { "string": text, "tags": [] },
        "type": note_type.unwrap_or("General"),
    });
    let resp: serde_json::Value = client.post("/api/notes/", &body).await?;
    extract_handle(resp)
}

pub async fn create_tag(
    client: &GrampsClient,
    name: &str,
    color: Option<&str>,
    priority: Option<i32>,
) -> Result<Handle> {
    let body = serde_json::json!({
        "name": name,
        "color": color.unwrap_or("#000000"),
        "priority": priority.unwrap_or(0),
    });
    let resp: serde_json::Value = client.post("/api/tags/", &body).await?;
    extract_handle(resp)
}

pub async fn create_citation(
    client: &GrampsClient,
    source_handle: &str,
    page: Option<&str>,
) -> Result<Handle> {
    let body = serde_json::json!({
        "source_handle": source_handle,
        "page": page.unwrap_or(""),
    });
    let resp: serde_json::Value = client.post("/api/citations/", &body).await?;
    extract_handle(resp)
}

pub async fn create_repository(
    client: &GrampsClient,
    name: &str,
    repo_type: Option<&str>,
) -> Result<Handle> {
    let body = serde_json::json!({
        "name": name,
        "type": repo_type.unwrap_or("Unknown"),
    });
    let resp: serde_json::Value = client.post("/api/repositories/", &body).await?;
    extract_handle(resp)
}

/// Create a media record from a server-side file path (metadata only).
///
/// POST /api/media/ treats the request body as binary file content, so path/desc/mime
/// cannot be set in one call. This function posts a placeholder and then PUTs the metadata.
pub async fn create_media_from_path(
    client: &GrampsClient,
    path: &str,
    description: Option<&str>,
    mime: Option<&str>,
) -> Result<Handle> {
    let resp: serde_json::Value = client.post("/api/media/", &serde_json::Value::Null).await?;
    let handle = extract_handle(resp)?;

    client
        .put::<_, serde_json::Value>(
            &format!("/api/media/{handle}"),
            &serde_json::json!({
                "handle": handle,
                "path": path,
                "desc": description.unwrap_or(""),
                "mime": mime.unwrap_or(""),
            }),
        )
        .await?;

    Ok(handle)
}

/// Download a file from a URL and upload it to Gramps as a media object.
///
/// POST /api/media/ with raw bytes creates the record and stores the file, populating
/// `path` and `checksum`. A follow-up GET+PUT is required to set description/mime without
/// overwriting those fields (Gramps Web API does a full object replace on PUT).
pub async fn create_media_from_url(
    client: &GrampsClient,
    url: &str,
    description: Option<&str>,
    mime: Option<&str>,
) -> Result<Handle> {
    let file_resp = client
        .http_client()
        .get(url)
        .send()
        .await
        .map_err(crate::client::Error::Http)?;

    let content_type = file_resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(';').next().unwrap_or(s).trim().to_string());

    let mime_type = mime
        .map(str::to_string)
        .or(content_type)
        .unwrap_or_else(|| "application/octet-stream".to_string());

    let bytes = file_resp
        .bytes()
        .await
        .map_err(crate::client::Error::Http)?;

    let resp: serde_json::Value = client
        .post_bytes("/api/media/", bytes.to_vec(), &mime_type)
        .await?;
    let handle = extract_handle(resp)?;

    // GET the full object so the PUT below doesn't overwrite path/checksum.
    let mut body = get::get_object_by_handle(client, "media", &handle).await?;
    body["mime"] = serde_json::json!(mime_type);
    if let Some(desc) = description {
        body["desc"] = serde_json::json!(desc);
    }
    client
        .put::<_, serde_json::Value>(&format!("/api/media/{handle}"), &body)
        .await?;

    Ok(handle)
}

fn extract_handle(resp: serde_json::Value) -> Result<Handle> {
    let obj = if let Some(arr) = resp.as_array() {
        // The newly created object has "old": null; updates have "old": {...}
        arr.iter()
            .find(|item| item["old"].is_null())
            .or_else(|| arr.first())
            .cloned()
            .unwrap_or(serde_json::Value::Null)
    } else {
        resp
    };
    obj["handle"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| ClientError::Parse(format!("no handle in response: {obj}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_handle_present() {
        let json = serde_json::json!({ "handle": "ABC123", "gramps_id": "I0001" });
        assert_eq!(extract_handle(json).unwrap(), "ABC123");
    }

    #[test]
    fn extract_handle_from_array() {
        let json = serde_json::json!([{ "handle": "ABC123", "type": "add", "old": null }]);
        assert_eq!(extract_handle(json).unwrap(), "ABC123");
    }

    #[test]
    fn extract_handle_prefers_new_object_over_updates() {
        let json = serde_json::json!([
            { "handle": "PARENT", "type": "modify", "old": { "handle": "PARENT" } },
            { "handle": "CHILD", "type": "add", "old": null }
        ]);
        assert_eq!(extract_handle(json).unwrap(), "CHILD");
    }

    #[test]
    fn extract_handle_missing_returns_parse_error() {
        let json = serde_json::json!({ "gramps_id": "I0001" });
        let err = extract_handle(json).unwrap_err();
        assert!(matches!(err, ClientError::Parse(_)));
        assert!(err.to_string().contains("no handle in response"));
    }

    #[test]
    fn extract_handle_null_returns_parse_error() {
        let json = serde_json::json!({ "handle": null });
        assert!(extract_handle(json).is_err());
    }
}
