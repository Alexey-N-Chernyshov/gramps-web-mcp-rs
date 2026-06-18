use crate::{
    client::{Error as ClientError, GrampsClient, Result},
    models::{
        event::CreateEventRequest, family::CreateFamilyRequest, person::CreatePersonRequest,
        place::CreatePlaceRequest, source::CreateSourceRequest, Handle,
    },
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
pub async fn create_media_from_path(
    client: &GrampsClient,
    path: &str,
    description: Option<&str>,
    mime: Option<&str>,
) -> Result<Handle> {
    let body = serde_json::json!({
        "path": path,
        "desc": description.unwrap_or(""),
        "mime": mime.unwrap_or(""),
    });
    let resp: serde_json::Value = client.post("/api/media/", &body).await?;
    extract_handle(resp)
}

/// Download a file from a URL and upload it to Gramps as a media object.
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

    let filename = url
        .split('/')
        .next_back()
        .filter(|s| !s.is_empty())
        .unwrap_or("file")
        .to_string();

    let bytes = file_resp
        .bytes()
        .await
        .map_err(crate::client::Error::Http)?;

    let part = reqwest::multipart::Part::bytes(bytes.to_vec())
        .file_name(filename)
        .mime_str(&mime_type)
        .map_err(crate::client::Error::Http)?;

    let mut form = reqwest::multipart::Form::new().part("file", part);
    if let Some(desc) = description {
        form = form.text("desc", desc.to_string());
    }

    let resp: serde_json::Value = client.post_multipart("/api/media/", form).await?;
    extract_handle(resp)
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
