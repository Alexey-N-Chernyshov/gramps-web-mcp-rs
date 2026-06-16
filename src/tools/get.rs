use crate::{
    client::{GrampsClient, Result},
    models::{Citation, Event, Family, Note, Person, Place, Repository, Source, Tag},
};

pub async fn get_person(client: &GrampsClient, handle: &str) -> Result<Person> {
    client.get(&format!("/api/people/{handle}")).await
}

pub async fn get_family(client: &GrampsClient, handle: &str) -> Result<Family> {
    client.get(&format!("/api/families/{handle}")).await
}

pub async fn get_event(client: &GrampsClient, handle: &str) -> Result<Event> {
    client.get(&format!("/api/events/{handle}")).await
}

pub async fn get_place(client: &GrampsClient, handle: &str) -> Result<Place> {
    client.get(&format!("/api/places/{handle}")).await
}

pub async fn get_source(client: &GrampsClient, handle: &str) -> Result<Source> {
    client.get(&format!("/api/sources/{handle}")).await
}

pub async fn get_citation(client: &GrampsClient, handle: &str) -> Result<Citation> {
    client.get(&format!("/api/citations/{handle}")).await
}

pub async fn get_note(client: &GrampsClient, handle: &str) -> Result<Note> {
    client.get(&format!("/api/notes/{handle}")).await
}

pub async fn get_media(client: &GrampsClient, handle: &str) -> Result<serde_json::Value> {
    client.get(&format!("/api/media/{handle}")).await
}

pub async fn get_repository(client: &GrampsClient, handle: &str) -> Result<Repository> {
    client.get(&format!("/api/repositories/{handle}")).await
}

pub async fn get_tag(client: &GrampsClient, handle: &str) -> Result<Tag> {
    client.get(&format!("/api/tags/{handle}")).await
}

pub async fn get_relations(
    client: &GrampsClient,
    handle1: &str,
    handle2: &str,
) -> Result<serde_json::Value> {
    client
        .get(&format!("/api/relations/{handle1}/{handle2}"))
        .await
}

pub async fn get_person_timeline(client: &GrampsClient, handle: &str) -> Result<serde_json::Value> {
    client.get(&format!("/api/people/{handle}/timeline")).await
}

pub async fn get_family_timeline(client: &GrampsClient, handle: &str) -> Result<serde_json::Value> {
    client
        .get(&format!("/api/families/{handle}/timeline"))
        .await
}

pub async fn get_event_span(
    client: &GrampsClient,
    handle1: &str,
    handle2: &str,
) -> Result<serde_json::Value> {
    client
        .get(&format!("/api/events/{handle1}/span/{handle2}"))
        .await
}

/// Returns tree-level statistics (person count, family count, etc.).
pub async fn get_tree_info(client: &GrampsClient) -> Result<serde_json::Value> {
    client.get("/api/metadata/").await
}
