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

fn matches_query_rule(goql_expr: &str) -> serde_json::Value {
    serde_json::json!({"rules": [{"name": "MatchesQuery", "values": [goql_expr]}]})
}

#[tokio::test]
async fn rules_filter_people_by_gramps_id() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let handle = create::create_person(client, CreatePersonRequest::default())
        .await
        .unwrap();

    let obj = get::get_object_by_handle(client, "people", &handle)
        .await
        .unwrap();
    let gramps_id = obj["gramps_id"].as_str().expect("gramps_id missing");

    let rules = matches_query_rule(&format!(r#"gramps_id == "{gramps_id}""#));
    let result = get::get_object_collection(client, "people", None, Some(&rules), None, None)
        .await
        .unwrap();

    assert!(result.is_array(), "rules filter should return an array");
    let items = result.as_array().unwrap();
    assert!(
        items
            .iter()
            .any(|p| p["handle"].as_str() == Some(handle.as_str())),
        "rules filter should find the created person"
    );

    // rules and pagesize work together
    let paged = get::get_object_collection(client, "people", None, Some(&rules), Some(1), Some(1))
        .await
        .unwrap();
    assert!(
        paged.is_array(),
        "rules + pagesize=1 should return an array"
    );
    assert!(
        paged.as_array().unwrap().len() <= 1,
        "pagesize=1 should cap results"
    );

    delete::delete_object(client, "people", &handle)
        .await
        .unwrap();
}

#[tokio::test]
async fn rules_filter_pagesize_without_page_still_caps_results() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    // Each person heads their own family (count(families) > 0) and has no parent
    // family (not exists(parent_families)) — more matches than the pagesize below.
    let mut people = Vec::new();
    let mut families = Vec::new();
    for _ in 0..4 {
        let person = create::create_person(client, CreatePersonRequest::default())
            .await
            .unwrap();
        let family = create::create_family(
            client,
            CreateFamilyRequest {
                father_handle: Some(person.clone()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        people.push(person);
        families.push(family);
    }

    let rules = matches_query_rule("count(families) > 0 and not exists(parent_families)");

    // Gramps Web defaults `page` to 0 when omitted, which disables pagination and
    // silently ignores `pagesize` — pagesize alone, with no page, must still cap.
    let paged = get::get_object_collection(client, "people", None, Some(&rules), None, Some(2))
        .await
        .unwrap();
    assert!(
        paged.as_array().unwrap().len() <= 2,
        "pagesize=2 without page should still cap results, got {}",
        paged
    );

    for family in families {
        delete::delete_object(client, "families", &family)
            .await
            .unwrap();
    }
    for person in people {
        delete::delete_object(client, "people", &person)
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn rules_filter_families_by_child_count() {
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

    let rules = matches_query_rule("count(children) > 0");
    let result = get::get_object_collection(client, "families", None, Some(&rules), None, None)
        .await
        .unwrap();

    assert!(result.is_array(), "rules filter should return an array");
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

#[tokio::test]
async fn get_filter_rules_lists_matches_query() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let result = get::get_filter_rules(client, "people").await.unwrap();

    let rules = result["rules"]
        .as_array()
        .expect("response should have a `rules` array");
    assert!(
        rules
            .iter()
            .any(|r| r["rule"].as_str() == Some("MatchesQuery")),
        "person rule catalog should include MatchesQuery"
    );
}
