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
        event::CreateEventRequest, family::CreateFamilyRequest, person::CreatePersonRequest,
        place::CreatePlaceRequest, source::CreateSourceRequest,
    },
    tools::{create, delete, get, merge},
};

#[tokio::test]
async fn merge_operations() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    // merge_person
    let p1 = create::create_person(client, CreatePersonRequest::default())
        .await
        .unwrap();
    let p2 = create::create_person(client, CreatePersonRequest::default())
        .await
        .unwrap();
    merge::merge_person(client, &p1, &p2, false).await.unwrap();
    assert!(matches!(
        get::get_object_by_handle(client, "people", &p2).await,
        Err(Error::NotFound(_))
    ));
    delete::delete_object(client, "people", &p1).await.unwrap();

    // merge_family
    let f1 = create::create_family(client, CreateFamilyRequest::default())
        .await
        .unwrap();
    let f2 = create::create_family(client, CreateFamilyRequest::default())
        .await
        .unwrap();
    merge::merge_family(client, &f1, &f2, None, None)
        .await
        .unwrap();
    assert!(matches!(
        get::get_object_by_handle(client, "families", &f2).await,
        Err(Error::NotFound(_))
    ));
    delete::delete_object(client, "families", &f1)
        .await
        .unwrap();

    // merge_event
    let e1 = create::create_event(client, CreateEventRequest::default())
        .await
        .unwrap();
    let e2 = create::create_event(client, CreateEventRequest::default())
        .await
        .unwrap();
    merge::merge_event(client, &e1, &e2).await.unwrap();
    assert!(matches!(
        get::get_object_by_handle(client, "events", &e2).await,
        Err(Error::NotFound(_))
    ));
    delete::delete_object(client, "events", &e1).await.unwrap();

    // merge_place
    let pl1 = create::create_place(client, CreatePlaceRequest::default())
        .await
        .unwrap();
    let pl2 = create::create_place(client, CreatePlaceRequest::default())
        .await
        .unwrap();
    merge::merge_place(client, &pl1, &pl2).await.unwrap();
    assert!(matches!(
        get::get_object_by_handle(client, "places", &pl2).await,
        Err(Error::NotFound(_))
    ));
    delete::delete_object(client, "places", &pl1).await.unwrap();

    // merge_note
    let n1 = create::create_note(client, "note one", None).await.unwrap();
    let n2 = create::create_note(client, "note two", None).await.unwrap();
    merge::merge_note(client, &n1, &n2).await.unwrap();
    assert!(matches!(
        get::get_object_by_handle(client, "notes", &n2).await,
        Err(Error::NotFound(_))
    ));
    delete::delete_object(client, "notes", &n1).await.unwrap();

    // merge_source + merge_citation + merge_repository
    let src1 = create::create_source(
        client,
        CreateSourceRequest {
            title: Some("Source One".to_string()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let src2 = create::create_source(
        client,
        CreateSourceRequest {
            title: Some("Source Two".to_string()),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let c1 = create::create_citation(client, &src1, None).await.unwrap();
    let c2 = create::create_citation(client, &src2, None).await.unwrap();
    merge::merge_citation(client, &c1, &c2).await.unwrap();
    assert!(matches!(
        get::get_object_by_handle(client, "citations", &c2).await,
        Err(Error::NotFound(_))
    ));
    delete::delete_object(client, "citations", &c1)
        .await
        .unwrap();

    merge::merge_source(client, &src1, &src2).await.unwrap();
    assert!(matches!(
        get::get_object_by_handle(client, "sources", &src2).await,
        Err(Error::NotFound(_))
    ));
    delete::delete_object(client, "sources", &src1)
        .await
        .unwrap();

    let r1 = create::create_repository(client, "Repo One", None)
        .await
        .unwrap();
    let r2 = create::create_repository(client, "Repo Two", None)
        .await
        .unwrap();
    merge::merge_repository(client, &r1, &r2).await.unwrap();
    assert!(matches!(
        get::get_object_by_handle(client, "repositories", &r2).await,
        Err(Error::NotFound(_))
    ));
    delete::delete_object(client, "repositories", &r1)
        .await
        .unwrap();

    // merge_media
    let m1 = create::create_media_from_path(client, "/tmp/a.jpg", None, None)
        .await
        .unwrap();
    let m2 = create::create_media_from_path(client, "/tmp/b.jpg", None, None)
        .await
        .unwrap();
    merge::merge_media(client, &m1, &m2).await.unwrap();
    assert!(matches!(
        get::get_object_by_handle(client, "media", &m2).await,
        Err(Error::NotFound(_))
    ));
    delete::delete_object(client, "media", &m1).await.unwrap();
}
