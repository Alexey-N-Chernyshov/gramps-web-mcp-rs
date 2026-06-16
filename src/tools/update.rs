use crate::client::{GrampsClient, Result};

pub async fn update_person(
    client: &GrampsClient,
    handle: &str,
    data: &serde_json::Value,
) -> Result<serde_json::Value> {
    client.put(&format!("/api/people/{handle}"), data).await
}

pub async fn update_family(
    client: &GrampsClient,
    handle: &str,
    data: &serde_json::Value,
) -> Result<serde_json::Value> {
    client.put(&format!("/api/families/{handle}"), data).await
}

pub async fn update_event(
    client: &GrampsClient,
    handle: &str,
    data: &serde_json::Value,
) -> Result<serde_json::Value> {
    client.put(&format!("/api/events/{handle}"), data).await
}

pub async fn update_place(
    client: &GrampsClient,
    handle: &str,
    data: &serde_json::Value,
) -> Result<serde_json::Value> {
    client.put(&format!("/api/places/{handle}"), data).await
}

pub async fn update_source(
    client: &GrampsClient,
    handle: &str,
    data: &serde_json::Value,
) -> Result<serde_json::Value> {
    client.put(&format!("/api/sources/{handle}"), data).await
}

pub async fn update_note(
    client: &GrampsClient,
    handle: &str,
    data: &serde_json::Value,
) -> Result<serde_json::Value> {
    client.put(&format!("/api/notes/{handle}"), data).await
}

pub async fn update_citation(
    client: &GrampsClient,
    handle: &str,
    data: &serde_json::Value,
) -> Result<serde_json::Value> {
    client.put(&format!("/api/citations/{handle}"), data).await
}

pub async fn update_repository(
    client: &GrampsClient,
    handle: &str,
    data: &serde_json::Value,
) -> Result<serde_json::Value> {
    client
        .put(&format!("/api/repositories/{handle}"), data)
        .await
}

pub async fn update_tag(
    client: &GrampsClient,
    handle: &str,
    data: &serde_json::Value,
) -> Result<serde_json::Value> {
    client.put(&format!("/api/tags/{handle}"), data).await
}

pub async fn update_media(
    client: &GrampsClient,
    handle: &str,
    data: &serde_json::Value,
) -> Result<serde_json::Value> {
    client.put(&format!("/api/media/{handle}"), data).await
}
