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

//! Source, citation, note, tag, and repository round trips.

#[path = "support/helpers.rs"]
mod common;

use gramps_web_mcp_rs::{
    client::Error,
    models::source::CreateSourceRequest,
    tools::{create, delete, get, update},
};

#[tokio::test]
async fn source_round_trip() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let handle = create::create_source(
        client,
        CreateSourceRequest {
            title: Some("Vital Records 1850".to_string()),
            author: Some("County Office".to_string()),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let source = get::get_object_by_handle(client, "sources", &handle)
        .await
        .unwrap();
    assert_eq!(source["title"].as_str(), Some("Vital Records 1850"));
    assert_eq!(source["author"].as_str(), Some("County Office"));

    let mut body = source.clone();
    body["title"] = serde_json::json!("Vital Records 1900");
    update::update_source(client, &handle, &body).await.unwrap();
    let updated = get::get_object_by_handle(client, "sources", &handle)
        .await
        .unwrap();
    assert_eq!(updated["title"].as_str(), Some("Vital Records 1900"));

    delete::delete_object(client, "sources", &handle)
        .await
        .unwrap();
    assert!(matches!(
        get::get_object_by_handle(client, "sources", &handle).await,
        Err(Error::NotFound(_))
    ));
}

#[tokio::test]
async fn citation_links_source() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let source_handle = create::create_source(
        client,
        CreateSourceRequest {
            title: Some("Parish Records".to_string()),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let citation_handle = create::create_citation(client, &source_handle, Some("p. 42"))
        .await
        .unwrap();

    let citation = get::get_object_by_handle(client, "citations", &citation_handle)
        .await
        .unwrap();
    assert_eq!(
        citation["source_handle"].as_str(),
        Some(source_handle.as_str())
    );
    assert_eq!(citation["page"].as_str(), Some("p. 42"));

    let mut body = citation.clone();
    body["page"] = serde_json::json!("p. 99");
    update::update_citation(client, &citation_handle, &body)
        .await
        .unwrap();
    let updated = get::get_object_by_handle(client, "citations", &citation_handle)
        .await
        .unwrap();
    assert_eq!(updated["page"].as_str(), Some("p. 99"));

    delete::delete_object(client, "citations", &citation_handle)
        .await
        .unwrap();
    delete::delete_object(client, "sources", &source_handle)
        .await
        .unwrap();
}

#[tokio::test]
async fn note_round_trip() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let handle = create::create_note(client, "Hello from test", Some("General"))
        .await
        .unwrap();

    let note = get::get_object_by_handle(client, "notes", &handle)
        .await
        .unwrap();
    assert_eq!(note["text"]["string"].as_str(), Some("Hello from test"));

    let mut body = note.clone();
    body["text"]["string"] = serde_json::json!("Updated note text");
    update::update_note(client, &handle, &body).await.unwrap();
    let updated = get::get_object_by_handle(client, "notes", &handle)
        .await
        .unwrap();
    assert_eq!(
        updated["text"]["string"].as_str(),
        Some("Updated note text")
    );

    delete::delete_object(client, "notes", &handle)
        .await
        .unwrap();
    assert!(matches!(
        get::get_object_by_handle(client, "notes", &handle).await,
        Err(Error::NotFound(_))
    ));
}

#[tokio::test]
async fn tag_round_trip() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let handle = create::create_tag(client, "Important", Some("#FF0000"), Some(1))
        .await
        .unwrap();

    let tag = get::get_object_by_handle(client, "tags", &handle)
        .await
        .unwrap();
    assert_eq!(tag["name"].as_str(), Some("Important"));
    assert_eq!(tag["color"].as_str(), Some("#FF0000"));

    let mut body = tag.clone();
    body["color"] = serde_json::json!("#00FF00");
    update::update_tag(client, &handle, &body).await.unwrap();
    let updated = get::get_object_by_handle(client, "tags", &handle)
        .await
        .unwrap();
    assert_eq!(updated["color"].as_str(), Some("#00FF00"));

    delete::delete_object(client, "tags", &handle)
        .await
        .unwrap();
    assert!(matches!(
        get::get_object_by_handle(client, "tags", &handle).await,
        Err(Error::NotFound(_))
    ));
}

#[tokio::test]
async fn repository_round_trip() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let handle = create::create_repository(client, "National Archives", Some("Archive"))
        .await
        .unwrap();

    let repo = get::get_object_by_handle(client, "repositories", &handle)
        .await
        .unwrap();
    assert_eq!(repo["name"].as_str(), Some("National Archives"));
    assert_eq!(repo["type"].as_str(), Some("Archive"));

    let mut body = repo.clone();
    body["name"] = serde_json::json!("State Archives");
    update::update_repository(client, &handle, &body)
        .await
        .unwrap();
    let updated = get::get_object_by_handle(client, "repositories", &handle)
        .await
        .unwrap();
    assert_eq!(updated["name"].as_str(), Some("State Archives"));

    delete::delete_object(client, "repositories", &handle)
        .await
        .unwrap();
    assert!(matches!(
        get::get_object_by_handle(client, "repositories", &handle).await,
        Err(Error::NotFound(_))
    ));
}
