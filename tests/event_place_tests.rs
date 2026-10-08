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

#[path = "support/helpers.rs"]
mod common;

use gramps_web_mcp_rs::{
    client::Error,
    models::{
        event::CreateEventRequest,
        place::{CreatePlaceRequest, PlaceName},
    },
    tools::{create, delete, get, update},
};

#[tokio::test]
async fn event_round_trip() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let handle = create::create_event(
        client,
        CreateEventRequest {
            event_type: Some(serde_json::json!("Birth")),
            description: Some("Test birth event".to_string()),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let event = get::get_object_by_handle(client, "events", &handle)
        .await
        .unwrap();
    assert_eq!(event["description"].as_str(), Some("Test birth event"));
    assert!(!event["type"].is_null());

    let mut body = event.clone();
    body["description"] = serde_json::json!("Updated description");
    update::update_event(client, &handle, &body).await.unwrap();
    let updated = get::get_object_by_handle(client, "events", &handle)
        .await
        .unwrap();
    assert_eq!(updated["description"].as_str(), Some("Updated description"));

    delete::delete_object(client, "events", &handle)
        .await
        .unwrap();
    assert!(matches!(
        get::get_object_by_handle(client, "events", &handle).await,
        Err(Error::NotFound(_))
    ));
}

#[tokio::test]
async fn place_round_trip() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let handle = create::create_place(
        client,
        CreatePlaceRequest {
            title: Some("Moscow".to_string()),
            name: Some(PlaceName {
                value: Some("Moscow".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let place = get::get_object_by_handle(client, "places", &handle)
        .await
        .unwrap();
    assert_eq!(place["title"].as_str(), Some("Moscow"));

    let mut body = place.clone();
    body["title"] = serde_json::json!("Saint Petersburg");
    update::update_place(client, &handle, &body).await.unwrap();
    let updated = get::get_object_by_handle(client, "places", &handle)
        .await
        .unwrap();
    assert_eq!(updated["title"].as_str(), Some("Saint Petersburg"));

    delete::delete_object(client, "places", &handle)
        .await
        .unwrap();
    assert!(matches!(
        get::get_object_by_handle(client, "places", &handle).await,
        Err(Error::NotFound(_))
    ));
}

#[tokio::test]
async fn event_span_between_two_events() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let handle1 = create::create_event(
        client,
        CreateEventRequest {
            event_type: Some(serde_json::json!("Birth")),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let handle2 = create::create_event(
        client,
        CreateEventRequest {
            event_type: Some(serde_json::json!("Death")),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    get::get_event_span(client, &handle1, &handle2)
        .await
        .unwrap();

    delete::delete_object(client, "events", &handle1)
        .await
        .unwrap();
    delete::delete_object(client, "events", &handle2)
        .await
        .unwrap();
}
