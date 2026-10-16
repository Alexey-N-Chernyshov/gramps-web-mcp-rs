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
        family::CreateFamilyRequest,
        person::{CreatePersonRequest, PersonName, Surname},
        source::CreateSourceRequest,
    },
    tools::{create, delete, get, update},
};

#[tokio::test]
async fn create_and_get_person_round_trip() {
    let fixture = common::TestFixture::new().await;

    let handle = create::create_person(
        &fixture.client,
        CreatePersonRequest {
            primary_name: Some(PersonName {
                first_name: Some("Ivan".to_string()),
                surname_list: vec![Surname {
                    surname: Some("Petrov".to_string()),
                    ..Default::default()
                }],
                name_type: Some("Birth Name".to_string()),
                ..Default::default()
            }),
            gender: Some(1),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let person = get::get_object_by_handle(&fixture.client, "people", &handle)
        .await
        .unwrap();
    assert_eq!(person["handle"].as_str(), Some(handle.as_str()));
    assert_eq!(person["primary_name"]["first_name"].as_str(), Some("Ivan"));
}

#[tokio::test]
async fn person_lifecycle() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let handle = create::create_person(
        client,
        CreatePersonRequest {
            primary_name: Some(PersonName {
                first_name: Some("Ivan".to_string()),
                surname_list: vec![Surname {
                    surname: Some("Sidorov".to_string()),
                    ..Default::default()
                }],
                name_type: Some("Birth Name".to_string()),
                ..Default::default()
            }),
            gender: Some(1),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let person = get::get_object_by_handle(client, "people", &handle)
        .await
        .unwrap();
    assert_eq!(person["primary_name"]["first_name"].as_str(), Some("Ivan"));

    let mut body = person.clone();
    body["primary_name"]["first_name"] = serde_json::json!("Petr");
    update::update_person(client, &handle, &body).await.unwrap();

    let updated = get::get_object_by_handle(client, "people", &handle)
        .await
        .unwrap();
    assert_eq!(updated["primary_name"]["first_name"].as_str(), Some("Petr"));

    delete::delete_object(client, "people", &handle)
        .await
        .unwrap();
    assert!(matches!(
        get::get_object_by_handle(client, "people", &handle).await,
        Err(Error::NotFound(_))
    ));
}

#[tokio::test]
async fn family_with_parents() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let father = create::create_person(
        client,
        CreatePersonRequest {
            gender: Some(1),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let mother = create::create_person(
        client,
        CreatePersonRequest {
            gender: Some(2),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let family_handle = create::create_family(
        client,
        CreateFamilyRequest {
            father_handle: Some(father.clone()),
            mother_handle: Some(mother.clone()),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let family = get::get_object_by_handle(client, "families", &family_handle)
        .await
        .unwrap();
    assert_eq!(family["father_handle"].as_str(), Some(father.as_str()));
    assert_eq!(family["mother_handle"].as_str(), Some(mother.as_str()));

    let new_mother = create::create_person(
        client,
        CreatePersonRequest {
            gender: Some(2),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let mut body = family.clone();
    body["mother_handle"] = serde_json::json!(new_mother);
    update::update_family(client, &family_handle, &body)
        .await
        .unwrap();
    let updated = get::get_object_by_handle(client, "families", &family_handle)
        .await
        .unwrap();
    assert_eq!(
        updated["mother_handle"].as_str(),
        Some(new_mother.as_str()),
        "mother_handle should be updated"
    );
    delete::delete_object(client, "people", &new_mother)
        .await
        .unwrap();
}

#[tokio::test]
async fn family_with_child() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let child = create::create_person(client, CreatePersonRequest::default())
        .await
        .unwrap();

    let family_handle = create::create_family(
        client,
        CreateFamilyRequest {
            child_ref_list: Some(vec![serde_json::json!({"ref": child})]),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let family = get::get_object_by_handle(client, "families", &family_handle)
        .await
        .unwrap();
    let children = family["child_ref_list"].as_array().unwrap();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0]["ref"].as_str(), Some(child.as_str()));
}

#[tokio::test]
async fn person_timeline() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let handle = create::create_person(client, CreatePersonRequest::default())
        .await
        .unwrap();

    let timeline = get::get_person_timeline(client, &handle).await.unwrap();
    assert!(timeline.is_array(), "timeline should be an array");

    delete::delete_object(client, "people", &handle)
        .await
        .unwrap();
}

#[tokio::test]
async fn family_timeline() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let family_handle = create::create_family(client, CreateFamilyRequest::default())
        .await
        .unwrap();

    let timeline = get::get_family_timeline(client, &family_handle)
        .await
        .unwrap();
    assert!(timeline.is_array(), "timeline should be an array");

    delete::delete_object(client, "families", &family_handle)
        .await
        .unwrap();
}

#[tokio::test]
async fn relations_between_parent_and_child() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let father = create::create_person(
        client,
        CreatePersonRequest {
            gender: Some(1),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let child = create::create_person(client, CreatePersonRequest::default())
        .await
        .unwrap();

    let family_handle = create::create_family(
        client,
        CreateFamilyRequest {
            father_handle: Some(father.clone()),
            child_ref_list: Some(vec![serde_json::json!({"ref": child})]),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    get::get_relations(client, &father, &child).await.unwrap();

    delete::delete_object(client, "families", &family_handle)
        .await
        .unwrap();
    delete::delete_object(client, "people", &father)
        .await
        .unwrap();
    delete::delete_object(client, "people", &child)
        .await
        .unwrap();
}

#[tokio::test]
async fn partial_update_preserves_unspecified_person_fields() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let note_handle = create::create_note(client, "A research note", None)
        .await
        .unwrap();
    let source_handle = create::create_source(
        client,
        CreateSourceRequest {
            title: Some("Census 1900".to_string()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let citation_handle = create::create_citation(client, &source_handle, Some("p. 12"))
        .await
        .unwrap();
    let event_handle = create::create_event(
        client,
        CreateEventRequest {
            event_type: Some(serde_json::json!("Birth")),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let handle = create::create_person(
        client,
        CreatePersonRequest {
            gender: Some(1),
            note_list: Some(vec![note_handle.clone()]),
            citation_list: Some(vec![citation_handle.clone()]),
            event_ref_list: Some(vec![
                serde_json::json!({"ref": event_handle, "role": "Primary"}),
            ]),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    // A partial update touching only `gender` must leave the other lists untouched.
    update::update_person(client, &handle, &serde_json::json!({"gender": 2}))
        .await
        .unwrap();

    let updated = get::get_object_by_handle(client, "people", &handle)
        .await
        .unwrap();
    assert_eq!(updated["gender"].as_i64(), Some(2));
    assert_eq!(
        updated["note_list"].as_array().unwrap(),
        &vec![serde_json::json!(note_handle)]
    );
    assert_eq!(
        updated["citation_list"].as_array().unwrap(),
        &vec![serde_json::json!(citation_handle)]
    );
    let event_refs = updated["event_ref_list"].as_array().unwrap();
    assert_eq!(event_refs.len(), 1);
    assert_eq!(event_refs[0]["ref"].as_str(), Some(event_handle.as_str()));
}

#[tokio::test]
async fn partial_update_replaces_list_when_included_in_data() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let note1 = create::create_note(client, "First note", None)
        .await
        .unwrap();
    let note2 = create::create_note(client, "Second note", None)
        .await
        .unwrap();

    let handle = create::create_person(
        client,
        CreatePersonRequest {
            note_list: Some(vec![note1.clone()]),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    update::update_person(
        client,
        &handle,
        &serde_json::json!({"note_list": [note2.clone()]}),
    )
    .await
    .unwrap();

    let updated = get::get_object_by_handle(client, "people", &handle)
        .await
        .unwrap();
    assert_eq!(
        updated["note_list"].as_array().unwrap(),
        &vec![serde_json::json!(note2)],
        "note_list should be replaced wholesale, not merged with the old one"
    );
}

#[tokio::test]
async fn update_rejects_mismatched_handle_in_data_without_writing() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let handle = create::create_person(
        client,
        CreatePersonRequest {
            gender: Some(1),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let err = update::update_person(
        client,
        &handle,
        &serde_json::json!({"handle": "some-other-handle", "gender": 2}),
    )
    .await
    .unwrap_err();
    assert!(matches!(err, Error::Validation(_)), "got: {err:?}");

    let unchanged = get::get_object_by_handle(client, "people", &handle)
        .await
        .unwrap();
    assert_eq!(
        unchanged["gender"].as_i64(),
        Some(1),
        "object must be left untouched when the write is rejected"
    );
}

#[tokio::test]
async fn update_rejects_empty_data() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let handle = create::create_person(client, CreatePersonRequest::default())
        .await
        .unwrap();

    let err = update::update_person(client, &handle, &serde_json::json!({}))
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Validation(_)), "got: {err:?}");
}

#[tokio::test]
async fn partial_update_preserves_family_note_list() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let note_handle = create::create_note(client, "Family note", None)
        .await
        .unwrap();
    let father = create::create_person(
        client,
        CreatePersonRequest {
            gender: Some(1),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let family_handle = create::create_family(
        client,
        CreateFamilyRequest {
            father_handle: Some(father),
            note_list: Some(vec![note_handle.clone()]),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let new_father = create::create_person(
        client,
        CreatePersonRequest {
            gender: Some(1),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    update::update_family(
        client,
        &family_handle,
        &serde_json::json!({"father_handle": new_father}),
    )
    .await
    .unwrap();

    let updated = get::get_object_by_handle(client, "families", &family_handle)
        .await
        .unwrap();
    assert_eq!(updated["father_handle"].as_str(), Some(new_father.as_str()));
    assert_eq!(
        updated["note_list"].as_array().unwrap(),
        &vec![serde_json::json!(note_handle)],
        "note_list should survive a partial update that only touches father_handle"
    );
}
