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
    models::{
        event::CreateEventRequest, family::CreateFamilyRequest, person::CreatePersonRequest,
        person::PersonName, place::CreatePlaceRequest, source::CreateSourceRequest,
    },
    tools::{create, delete, get, search},
};

#[tokio::test]
async fn search_endpoints_return_ok() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let person = create::create_person(
        client,
        CreatePersonRequest {
            primary_name: Some(PersonName {
                first_name: Some("Searchable".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let source = create::create_source(
        client,
        CreateSourceRequest {
            title: Some("Search Source".to_string()),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let citation = create::create_citation(client, &source, Some("p. 1"))
        .await
        .unwrap();

    let event = create::create_event(
        client,
        CreateEventRequest {
            event_type: Some(serde_json::json!("Birth")),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let place = create::create_place(
        client,
        CreatePlaceRequest {
            title: Some("Search Place".to_string()),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let family = create::create_family(client, CreateFamilyRequest::default())
        .await
        .unwrap();

    let note = create::create_note(client, "search note text", None)
        .await
        .unwrap();

    let tag = create::create_tag(client, "SearchTag", None, None)
        .await
        .unwrap();

    let repo = create::create_repository(client, "Search Repo", None)
        .await
        .unwrap();

    let media = create::create_media_from_path(client, "/tmp/search.jpg", None, None)
        .await
        .unwrap();

    // Each type must be reachable and return a JSON array.
    // If a type is not yet indexed, the server returns Ok([]) — still valid.
    macro_rules! assert_search {
        ($query:expr, $type:expr) => {
            let r = search::search(client, $query, $type, None, None)
                .await
                .unwrap_or_else(|e| panic!("search({:?}, {:?}) failed: {e}", $query, $type));
            assert!(
                r.is_array(),
                "search({:?}, {:?}) should return an array",
                $query,
                $type
            );
        };
    }

    assert_search!("Searchable", Some("person"));
    assert_search!("Search Source", Some("source"));
    assert_search!("citation", Some("citation"));
    assert_search!("Birth", Some("event"));
    assert_search!("Search Place", Some("place"));
    assert_search!("family", Some("family"));
    assert_search!("note", Some("note"));
    assert_search!("SearchTag", Some("tag"));
    assert_search!("Search Repo", Some("repository"));
    assert_search!("search", Some("media"));
    assert_search!("Search", None::<&str>);

    // Cleanup
    delete::delete_object(client, "people", &person)
        .await
        .unwrap();
    delete::delete_object(client, "citations", &citation)
        .await
        .unwrap();
    delete::delete_object(client, "sources", &source)
        .await
        .unwrap();
    delete::delete_object(client, "events", &event)
        .await
        .unwrap();
    delete::delete_object(client, "places", &place)
        .await
        .unwrap();
    delete::delete_object(client, "families", &family)
        .await
        .unwrap();
    delete::delete_object(client, "notes", &note).await.unwrap();
    delete::delete_object(client, "tags", &tag).await.unwrap();
    delete::delete_object(client, "repositories", &repo)
        .await
        .unwrap();
    delete::delete_object(client, "media", &media)
        .await
        .unwrap();
}

#[tokio::test]
async fn get_object_collection_pagination_and_gramps_id() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let h1 = create::create_person(client, CreatePersonRequest::default())
        .await
        .unwrap();
    let h2 = create::create_person(client, CreatePersonRequest::default())
        .await
        .unwrap();

    // plain collection browse returns an array
    let all = get::get_object_collection(client, "people", None, None, None, None)
        .await
        .unwrap();
    assert!(
        all.is_array(),
        "collection without params should be an array"
    );

    // pagination: page=1 pagesize=1 must return at most 1 item
    let page1 = get::get_object_collection(client, "people", None, None, Some(1), Some(1))
        .await
        .unwrap();
    assert!(page1.is_array());
    assert!(
        page1.as_array().unwrap().len() <= 1,
        "pagesize=1 should return at most 1 item"
    );

    // gramps_id lookup: fetch the gramps_id of h1 then query by it
    let obj = get::get_object_by_handle(client, "people", &h1)
        .await
        .unwrap();
    let gramps_id = obj["gramps_id"].as_str().expect("gramps_id missing");
    let by_id = get::get_object_collection(client, "people", Some(gramps_id), None, None, None)
        .await
        .unwrap();
    assert!(by_id.is_array());
    let items = by_id.as_array().unwrap();
    assert_eq!(
        items.len(),
        1,
        "gramps_id lookup should return exactly 1 item"
    );
    assert_eq!(items[0]["handle"].as_str(), Some(h1.as_str()));

    delete::delete_object(client, "people", &h1).await.unwrap();
    delete::delete_object(client, "people", &h2).await.unwrap();
}

#[tokio::test]
async fn search_pagination() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    // Create two people with the same distinctive name so search finds at least 2 results.
    let h1 = create::create_person(
        client,
        CreatePersonRequest {
            primary_name: Some(PersonName {
                first_name: Some("Pagination".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let h2 = create::create_person(
        client,
        CreatePersonRequest {
            primary_name: Some(PersonName {
                first_name: Some("Pagination".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    // pagesize=1 must return at most 1 result
    let page1 = search::search(client, "Pagination", Some("person"), Some(1), Some(1))
        .await
        .unwrap();
    assert!(page1.is_array(), "paginated search should return an array");
    assert!(
        page1.as_array().unwrap().len() <= 1,
        "pagesize=1 should return at most 1 result, got {}",
        page1.as_array().unwrap().len()
    );

    // page=2 pagesize=1 — second page should also be an array (may be empty if not yet indexed)
    let page2 = search::search(client, "Pagination", Some("person"), Some(2), Some(1))
        .await
        .unwrap();
    assert!(page2.is_array(), "page 2 should return an array");

    // No pagination params — should also be fine (backward compat)
    let all = search::search(client, "Pagination", Some("person"), None, None)
        .await
        .unwrap();
    assert!(
        all.is_array(),
        "search without pagination should return an array"
    );

    delete::delete_object(client, "people", &h1).await.unwrap();
    delete::delete_object(client, "people", &h2).await.unwrap();
}

#[tokio::test]
async fn oql_filter_people_by_gramps_id() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let handle = create::create_person(client, CreatePersonRequest::default())
        .await
        .unwrap();

    let obj = get::get_object_by_handle(client, "people", &handle)
        .await
        .unwrap();
    let gramps_id = obj["gramps_id"].as_str().expect("gramps_id missing");

    let query = format!(r#"person.gramps_id == "{gramps_id}""#);
    let result = get::get_object_collection(client, "people", None, Some(&query), None, None)
        .await
        .unwrap();

    assert!(result.is_array(), "oql filter should return an array");
    let items = result.as_array().unwrap();
    assert!(
        items
            .iter()
            .any(|p| p["handle"].as_str() == Some(handle.as_str())),
        "oql filter should find the created person"
    );

    // oql and pagesize work together
    let paged = get::get_object_collection(client, "people", None, Some(&query), Some(1), Some(1))
        .await
        .unwrap();
    assert!(paged.is_array(), "oql + pagesize=1 should return an array");
    assert!(
        paged.as_array().unwrap().len() <= 1,
        "pagesize=1 should cap results"
    );

    delete::delete_object(client, "people", &handle)
        .await
        .unwrap();
}

#[tokio::test]
async fn oql_filter_families_by_child_count() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let child = create::create_person(client, CreatePersonRequest::default())
        .await
        .unwrap();

    let empty_family = create::create_family(client, CreateFamilyRequest::default())
        .await
        .unwrap();

    let family_with_child = create::create_family(
        client,
        CreateFamilyRequest {
            child_ref_list: Some(vec![serde_json::json!({"ref": child})]),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let result = get::get_object_collection(
        client,
        "families",
        None,
        Some("len(family.get_child_ref_list()) > 0"),
        None,
        None,
    )
    .await
    .unwrap();

    assert!(result.is_array(), "oql filter should return an array");
    let items = result.as_array().unwrap();
    assert!(
        items
            .iter()
            .any(|f| f["handle"].as_str() == Some(family_with_child.as_str())),
        "family with child should be in results"
    );
    assert!(
        !items
            .iter()
            .any(|f| f["handle"].as_str() == Some(empty_family.as_str())),
        "family without children should not be in results"
    );

    delete::delete_object(client, "families", &empty_family)
        .await
        .unwrap();
    delete::delete_object(client, "families", &family_with_child)
        .await
        .unwrap();
    delete::delete_object(client, "people", &child)
        .await
        .unwrap();
}
