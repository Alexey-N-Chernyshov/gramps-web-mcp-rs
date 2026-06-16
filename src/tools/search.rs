use crate::client::{GrampsClient, Result};

pub async fn find_person(client: &GrampsClient, query: &str) -> Result<serde_json::Value> {
    let path = format!(
        "/api/search/?query={}&type=person",
        urlencoding::encode(query)
    );
    client.get(&path).await
}

pub async fn find_family(client: &GrampsClient, query: &str) -> Result<serde_json::Value> {
    let path = format!(
        "/api/search/?query={}&type=family",
        urlencoding::encode(query)
    );
    client.get(&path).await
}

pub async fn find_event(client: &GrampsClient, query: &str) -> Result<serde_json::Value> {
    let path = format!(
        "/api/search/?query={}&type=event",
        urlencoding::encode(query)
    );
    client.get(&path).await
}

pub async fn find_place(client: &GrampsClient, query: &str) -> Result<serde_json::Value> {
    let path = format!(
        "/api/search/?query={}&type=place",
        urlencoding::encode(query)
    );
    client.get(&path).await
}

pub async fn find_source(client: &GrampsClient, query: &str) -> Result<serde_json::Value> {
    let path = format!(
        "/api/search/?query={}&type=source",
        urlencoding::encode(query)
    );
    client.get(&path).await
}

pub async fn find_citation(client: &GrampsClient, query: &str) -> Result<serde_json::Value> {
    let path = format!(
        "/api/search/?query={}&type=citation",
        urlencoding::encode(query)
    );
    client.get(&path).await
}

pub async fn find_media(client: &GrampsClient, query: &str) -> Result<serde_json::Value> {
    let path = format!(
        "/api/search/?query={}&type=media",
        urlencoding::encode(query)
    );
    client.get(&path).await
}

pub async fn find_repository(client: &GrampsClient, query: &str) -> Result<serde_json::Value> {
    let path = format!(
        "/api/search/?query={}&type=repository",
        urlencoding::encode(query)
    );
    client.get(&path).await
}

pub async fn find_note(client: &GrampsClient, query: &str) -> Result<serde_json::Value> {
    let path = format!(
        "/api/search/?query={}&type=note",
        urlencoding::encode(query)
    );
    client.get(&path).await
}

pub async fn find_tag(client: &GrampsClient, query: &str) -> Result<serde_json::Value> {
    let path = format!("/api/search/?query={}&type=tag", urlencoding::encode(query));
    client.get(&path).await
}

pub async fn find_anything(client: &GrampsClient, query: &str) -> Result<serde_json::Value> {
    let path = format!("/api/search/?query={}", urlencoding::encode(query));
    client.get(&path).await
}
