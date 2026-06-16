use crate::client::{GrampsClient, Result};

pub async fn delete_person(client: &GrampsClient, handle: &str) -> Result<()> {
    client.delete(&format!("/api/people/{handle}")).await
}

pub async fn delete_family(client: &GrampsClient, handle: &str) -> Result<()> {
    client.delete(&format!("/api/families/{handle}")).await
}

pub async fn delete_event(client: &GrampsClient, handle: &str) -> Result<()> {
    client.delete(&format!("/api/events/{handle}")).await
}

pub async fn delete_place(client: &GrampsClient, handle: &str) -> Result<()> {
    client.delete(&format!("/api/places/{handle}")).await
}

pub async fn delete_source(client: &GrampsClient, handle: &str) -> Result<()> {
    client.delete(&format!("/api/sources/{handle}")).await
}

pub async fn delete_citation(client: &GrampsClient, handle: &str) -> Result<()> {
    client.delete(&format!("/api/citations/{handle}")).await
}

pub async fn delete_repository(client: &GrampsClient, handle: &str) -> Result<()> {
    client.delete(&format!("/api/repositories/{handle}")).await
}

pub async fn delete_note(client: &GrampsClient, handle: &str) -> Result<()> {
    client.delete(&format!("/api/notes/{handle}")).await
}

pub async fn delete_tag(client: &GrampsClient, handle: &str) -> Result<()> {
    client.delete(&format!("/api/tags/{handle}")).await
}

pub async fn delete_media(client: &GrampsClient, handle: &str) -> Result<()> {
    client.delete(&format!("/api/media/{handle}")).await
}
