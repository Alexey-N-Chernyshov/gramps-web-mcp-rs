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
    client::GrampsClient,
    models::{
        event::CreateEventRequest, family::CreateFamilyRequest, person::CreatePersonRequest,
        person::PersonName, place::CreatePlaceRequest, source::CreateSourceRequest, Handle,
    },
    tools::{create, delete, get, search, update},
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
    let all = get::get_object_collection(
        client,
        "people",
        get::CollectionQuery {
            gramps_id: None,
            rules: None,
            page: None,
            pagesize: None,
            handles: None,
            sort: None,
        },
        get::ReadOptions::default(),
    )
    .await
    .unwrap();
    assert!(
        all.is_array(),
        "collection without params should be an array"
    );

    // pagination: page=1 pagesize=1 must return at most 1 item
    let page1 = get::get_object_collection(
        client,
        "people",
        get::CollectionQuery {
            gramps_id: None,
            rules: None,
            page: Some(1),
            pagesize: Some(1),
            handles: None,
            sort: None,
        },
        get::ReadOptions::default(),
    )
    .await
    .unwrap();
    assert!(page1.is_array());
    assert!(
        page1.as_array().unwrap().len() <= 1,
        "pagesize=1 should return at most 1 item"
    );

    // gramps_id lookup: fetch the gramps_id of h1 then query by it
    let obj = get::get_object_by_handle(client, "people", &h1, get::ReadOptions::default())
        .await
        .unwrap();
    let gramps_id = obj["gramps_id"].as_str().expect("gramps_id missing");
    let by_id = get::get_object_collection(
        client,
        "people",
        get::CollectionQuery {
            gramps_id: Some(gramps_id),
            rules: None,
            page: None,
            pagesize: None,
            handles: None,
            sort: None,
        },
        get::ReadOptions::default(),
    )
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
async fn get_object_by_gramps_id_returns_single_object_not_array() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let handle = create::create_person(client, CreatePersonRequest::default())
        .await
        .unwrap();
    let obj = get::get_object_by_handle(client, "people", &handle, get::ReadOptions::default())
        .await
        .unwrap();
    let gramps_id = obj["gramps_id"].as_str().expect("gramps_id missing");

    let by_id =
        get::get_object_by_gramps_id(client, "people", gramps_id, get::ReadOptions::default())
            .await
            .unwrap();
    assert!(
        by_id.is_object(),
        "get_object_by_gramps_id should return a single object, not an array; got {by_id}"
    );
    assert_eq!(by_id["handle"].as_str(), Some(handle.as_str()));

    delete::delete_object(client, "people", &handle)
        .await
        .unwrap();
}

#[tokio::test]
async fn get_object_by_gramps_id_missing_returns_not_found() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let err = get::get_object_by_gramps_id(
        client,
        "people",
        "NONEXISTENT_GRAMPS_ID",
        get::ReadOptions::default(),
    )
    .await
    .unwrap_err();
    assert!(
        matches!(err, gramps_web_mcp_rs::client::Error::NotFound(_)),
        "got: {err:?}"
    );
}

// The ambiguous-match (gramps_id matching >1 object) branch is covered by a unit test on
// `resolve_gramps_id_match` in src/tools/get.rs — Gramps Web enforces gramps_id uniqueness
// server-side, so a genuine collision isn't reproducible through the live API.

#[tokio::test]
async fn get_object_by_handle_with_keys_returns_only_requested_fields() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let handle = create::create_person(client, CreatePersonRequest::default())
        .await
        .unwrap();

    let keys = vec!["gramps_id".to_string(), "primary_name".to_string()];
    let obj = get::get_object_by_handle(
        client,
        "people",
        &handle,
        get::ReadOptions {
            keys: Some(&keys),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let fields: Vec<&str> = obj
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert!(
        fields.iter().all(|f| keys.iter().any(|k| k == f)),
        "response should only contain requested keys {keys:?}, got {fields:?}"
    );
    assert!(obj.get("gramps_id").is_some(), "got {obj}");
    assert!(
        obj.get("address_list").is_none(),
        "address_list wasn't requested, got {obj}"
    );

    delete::delete_object(client, "people", &handle)
        .await
        .unwrap();
}

#[tokio::test]
async fn get_object_by_gramps_id_with_keys_returns_only_requested_fields() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let handle = create::create_person(client, CreatePersonRequest::default())
        .await
        .unwrap();
    let full = get::get_object_by_handle(client, "people", &handle, get::ReadOptions::default())
        .await
        .unwrap();
    let gramps_id = full["gramps_id"].as_str().expect("gramps_id missing");

    let keys = vec!["handle".to_string()];
    let obj = get::get_object_by_gramps_id(
        client,
        "people",
        gramps_id,
        get::ReadOptions {
            keys: Some(&keys),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let fields: Vec<&str> = obj
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert!(
        fields.iter().all(|f| keys.iter().any(|k| k == f)),
        "response should only contain requested keys {keys:?}, got {fields:?}"
    );
    assert_eq!(
        obj.get("handle").and_then(|h| h.as_str()),
        Some(handle.as_str())
    );

    delete::delete_object(client, "people", &handle)
        .await
        .unwrap();
}

#[tokio::test]
async fn list_objects_with_rules_and_keys_restricts_every_item() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let handle = create::create_person(client, CreatePersonRequest::default())
        .await
        .unwrap();
    let full = get::get_object_by_handle(client, "people", &handle, get::ReadOptions::default())
        .await
        .unwrap();
    let gramps_id = full["gramps_id"].as_str().expect("gramps_id missing");

    let rules = matches_query_rule(&format!(r#"gramps_id == "{gramps_id}""#));
    let keys = vec!["gramps_id".to_string(), "handle".to_string()];
    let result = get::get_object_collection(
        client,
        "people",
        get::CollectionQuery {
            gramps_id: None,
            rules: Some(&rules),
            page: None,
            pagesize: None,
            handles: None,
            sort: None,
        },
        get::ReadOptions {
            keys: Some(&keys),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let items = result.as_array().expect("expected an array");
    assert!(!items.is_empty(), "expected at least one match");
    for item in items {
        let fields: Vec<&str> = item
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert!(
            fields.iter().all(|f| keys.iter().any(|k| k == f)),
            "every item should only contain requested keys {keys:?}, got {fields:?}"
        );
    }
    assert!(
        items
            .iter()
            .any(|p| p["handle"].as_str() == Some(handle.as_str())),
        "rules filter should find the created person"
    );

    delete::delete_object(client, "people", &handle)
        .await
        .unwrap();
}

#[tokio::test]
async fn list_objects_with_empty_keys_returns_full_objects() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let handle = create::create_person(client, CreatePersonRequest::default())
        .await
        .unwrap();
    let full_opts = get::ReadOptions {
        strip: Some(false),
        ..Default::default()
    };
    let full = get::get_object_by_handle(client, "people", &handle, full_opts)
        .await
        .unwrap();
    let gramps_id = full["gramps_id"].as_str().expect("gramps_id missing");

    let empty_keys: Vec<String> = vec![];
    let result = get::get_object_collection(
        client,
        "people",
        get::CollectionQuery {
            gramps_id: Some(gramps_id),
            rules: None,
            page: None,
            pagesize: None,
            handles: None,
            sort: None,
        },
        get::ReadOptions {
            keys: Some(&empty_keys),
            strip: Some(false),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let items = result.as_array().expect("expected an array");
    assert_eq!(items.len(), 1);
    assert!(
        items[0].get("primary_name").is_some(),
        "empty keys should return the full object, got {}",
        items[0]
    );

    delete::delete_object(client, "people", &handle)
        .await
        .unwrap();
}

#[tokio::test]
async fn get_object_by_handle_strips_empty_fields_by_default() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    // note_list is an empty list — exactly what stripping should drop. gender/private are
    // explicit falsy-but-not-empty values (Gramps Web defaults an unset gender to 2, not 0).
    let handle = create::create_person(
        client,
        CreatePersonRequest {
            gender: Some(0),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    // No `strip` at all — ReadOptions::default() — must still strip; that's the tools'
    // own default, independent of Gramps Web's own default (strip=false).
    let default_opts =
        get::get_object_by_handle(client, "people", &handle, get::ReadOptions::default())
            .await
            .unwrap();
    assert!(
        default_opts.get("note_list").is_none(),
        "empty fields should be omitted by default (no `strip` argument at all), got {default_opts}"
    );

    let explicit_true = get::get_object_by_handle(
        client,
        "people",
        &handle,
        get::ReadOptions {
            strip: Some(true),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert!(
        explicit_true.get("note_list").is_none(),
        "strip=true should omit the empty note_list field, got {explicit_true}"
    );

    let strip_false = get::get_object_by_handle(
        client,
        "people",
        &handle,
        get::ReadOptions {
            strip: Some(false),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(
        strip_false.get("note_list"),
        Some(&serde_json::json!([])),
        "strip=false should opt back into the full object, including empty note_list"
    );
    assert_eq!(
        strip_false.get("gender"),
        Some(&serde_json::json!(0)),
        "gender=0 (unknown) is falsy but not empty — must never be stripped, got {strip_false}"
    );
    assert_eq!(
        strip_false.get("private"),
        Some(&serde_json::json!(false)),
        "private=false is falsy but not empty — must never be stripped, got {strip_false}"
    );

    // And the same falsy-but-not-empty fields must survive the default (stripped) path too.
    assert_eq!(
        default_opts.get("gender"),
        Some(&serde_json::json!(0)),
        "gender=0 must not be stripped even when strip is on by default, got {default_opts}"
    );
    assert_eq!(
        default_opts.get("private"),
        Some(&serde_json::json!(false)),
        "private=false must not be stripped even when strip is on by default, got {default_opts}"
    );

    delete::delete_object(client, "people", &handle)
        .await
        .unwrap();
}

#[tokio::test]
async fn list_objects_with_rules_keys_and_strip_combined() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    // primary_name/gramps_id are non-empty; note_list is left empty.
    let handle = create::create_person(
        client,
        CreatePersonRequest {
            primary_name: Some(PersonName {
                first_name: Some("StripTest".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let full = get::get_object_by_handle(client, "people", &handle, get::ReadOptions::default())
        .await
        .unwrap();
    let gramps_id = full["gramps_id"].as_str().expect("gramps_id missing");

    let rules = matches_query_rule(&format!(r#"gramps_id == "{gramps_id}""#));
    let keys = vec![
        "gramps_id".to_string(),
        "primary_name".to_string(),
        "note_list".to_string(),
    ];
    let result = get::get_object_collection(
        client,
        "people",
        get::CollectionQuery {
            gramps_id: None,
            rules: Some(&rules),
            page: None,
            pagesize: None,
            handles: None,
            sort: None,
        },
        get::ReadOptions {
            keys: Some(&keys),
            strip: Some(true),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let items = result.as_array().expect("expected an array");
    assert_eq!(items.len(), 1);
    let item = &items[0];
    let fields: Vec<&str> = item
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert!(
        fields.iter().all(|f| keys.iter().any(|k| k == f)),
        "every field should be one of the requested keys {keys:?}, got {fields:?}"
    );
    assert!(
        item.get("note_list").is_none(),
        "strip=true should drop the empty note_list even though it was requested via keys, got {item}"
    );
    assert!(item.get("gramps_id").is_some(), "got {item}");
    assert!(item.get("primary_name").is_some(), "got {item}");

    delete::delete_object(client, "people", &handle)
        .await
        .unwrap();
}

#[tokio::test]
async fn get_object_by_handle_with_profile_self_includes_name_birth_death() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let handle = create::create_person(
        client,
        CreatePersonRequest {
            primary_name: Some(PersonName {
                first_name: Some("ProfileTest".to_string()),
                ..Default::default()
            }),
            gender: Some(1),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    // strip=false: strip (on by default) would otherwise recurse into `profile` itself and
    // drop its empty `birth`/`death` summaries (this person has no birth/death event), which
    // is exactly what this test wants to see.
    let profile_sections = vec!["self".to_string()];
    let obj = get::get_object_by_handle(
        client,
        "people",
        &handle,
        get::ReadOptions {
            profile: Some(&profile_sections),
            strip: Some(false),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let profile = obj
        .get("profile")
        .expect("profile=[\"self\"] should add a `profile` field")
        .as_object()
        .expect("profile should be an object");
    assert_eq!(
        profile.get("name_given").and_then(|v| v.as_str()),
        Some("ProfileTest"),
        "got profile: {profile:?}"
    );
    assert_eq!(
        profile.get("sex").and_then(|v| v.as_str()),
        Some("M"),
        "got profile: {profile:?}"
    );
    assert!(
        profile.contains_key("birth") && profile.contains_key("death"),
        "self profile should include birth/death summaries, got {profile:?}"
    );

    delete::delete_object(client, "people", &handle)
        .await
        .unwrap();
}

#[tokio::test]
async fn get_object_by_handle_with_keys_and_profile_includes_both() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let handle = create::create_person(client, CreatePersonRequest::default())
        .await
        .unwrap();

    let keys = vec!["gramps_id".to_string()];
    let profile_sections = vec!["self".to_string()];
    let obj = get::get_object_by_handle(
        client,
        "people",
        &handle,
        get::ReadOptions {
            keys: Some(&keys),
            profile: Some(&profile_sections),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    assert!(
        obj.get("gramps_id").is_some(),
        "requested key gramps_id missing, got {obj}"
    );
    assert!(
        obj.get("profile").is_some(),
        "keys=[\"gramps_id\"] (without \"profile\") must not silently strip the requested \
         profile summary back out, got {obj}"
    );
    let fields: Vec<&str> = obj
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert!(
        fields.iter().all(|f| *f == "gramps_id" || *f == "profile"),
        "response should only contain the requested key plus the auto-added profile, got {fields:?}"
    );

    delete::delete_object(client, "people", &handle)
        .await
        .unwrap();
}

#[tokio::test]
async fn list_objects_with_rules_and_profile() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let handle = create::create_person(
        client,
        CreatePersonRequest {
            primary_name: Some(PersonName {
                first_name: Some("ListProfileTest".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let full = get::get_object_by_handle(client, "people", &handle, get::ReadOptions::default())
        .await
        .unwrap();
    let gramps_id = full["gramps_id"].as_str().expect("gramps_id missing");

    let rules = matches_query_rule(&format!(r#"gramps_id == "{gramps_id}""#));
    let profile_sections = vec!["self".to_string()];
    let result = get::get_object_collection(
        client,
        "people",
        get::CollectionQuery {
            gramps_id: None,
            rules: Some(&rules),
            page: None,
            pagesize: None,
            handles: None,
            sort: None,
        },
        get::ReadOptions {
            profile: Some(&profile_sections),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let items = result.as_array().expect("expected an array");
    assert_eq!(items.len(), 1);
    assert_eq!(
        items[0]["profile"]["name_given"].as_str(),
        Some("ListProfileTest"),
        "got {}",
        items[0]
    );

    delete::delete_object(client, "people", &handle)
        .await
        .unwrap();
}

#[tokio::test]
async fn list_objects_with_handles_preserves_requested_order() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let h1 = create::create_person(client, CreatePersonRequest::default())
        .await
        .unwrap();
    let h2 = create::create_person(client, CreatePersonRequest::default())
        .await
        .unwrap();
    let h3 = create::create_person(client, CreatePersonRequest::default())
        .await
        .unwrap();

    // Deliberately out of creation order.
    let handles = vec![h3.clone(), h1.clone(), h2.clone()];
    let result = get::get_object_collection(
        client,
        "people",
        get::CollectionQuery {
            handles: Some(&handles),
            ..Default::default()
        },
        get::ReadOptions::default(),
    )
    .await
    .unwrap();

    let items = result.as_array().expect("expected an array");
    let returned_handles: Vec<&str> = items
        .iter()
        .map(|i| i["handle"].as_str().expect("handle missing"))
        .collect();
    assert_eq!(
        returned_handles,
        vec![h3.as_str(), h1.as_str(), h2.as_str()],
        "handles should come back in the requested order, not creation order"
    );

    delete::delete_object(client, "people", &h1).await.unwrap();
    delete::delete_object(client, "people", &h2).await.unwrap();
    delete::delete_object(client, "people", &h3).await.unwrap();
}

#[tokio::test]
async fn list_objects_with_handles_skips_unknown_handle() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let h1 = create::create_person(client, CreatePersonRequest::default())
        .await
        .unwrap();
    let h2 = create::create_person(client, CreatePersonRequest::default())
        .await
        .unwrap();

    let handles = vec![h1.clone(), "NONEXISTENT_HANDLE".to_string(), h2.clone()];
    let result = get::get_object_collection(
        client,
        "people",
        get::CollectionQuery {
            handles: Some(&handles),
            ..Default::default()
        },
        get::ReadOptions::default(),
    )
    .await
    .unwrap();

    let items = result.as_array().expect("expected an array");
    assert_eq!(
        items.len(),
        2,
        "the unknown handle should be silently skipped rather than erroring, got {items:?}"
    );
    let returned_handles: Vec<&str> = items
        .iter()
        .map(|i| i["handle"].as_str().expect("handle missing"))
        .collect();
    assert_eq!(returned_handles, vec![h1.as_str(), h2.as_str()]);

    delete::delete_object(client, "people", &h1).await.unwrap();
    delete::delete_object(client, "people", &h2).await.unwrap();
}

#[tokio::test]
async fn list_objects_with_handles_and_keys_restricts_every_item() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let h1 = create::create_person(client, CreatePersonRequest::default())
        .await
        .unwrap();
    let h2 = create::create_person(client, CreatePersonRequest::default())
        .await
        .unwrap();

    let handles = vec![h1.clone(), h2.clone()];
    let keys = vec!["gramps_id".to_string()];
    let result = get::get_object_collection(
        client,
        "people",
        get::CollectionQuery {
            handles: Some(&handles),
            ..Default::default()
        },
        get::ReadOptions {
            keys: Some(&keys),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let items = result.as_array().expect("expected an array");
    assert_eq!(items.len(), 2);
    for item in items {
        let fields: Vec<&str> = item
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            fields,
            vec!["gramps_id"],
            "handles + keys should still restrict each item to the requested fields, got {fields:?}"
        );
    }

    delete::delete_object(client, "people", &h1).await.unwrap();
    delete::delete_object(client, "people", &h2).await.unwrap();
}

/// Creates a person with a birth event dated `year` so `sort=["birth"]`/`["-birth"]` has
/// something real to compare.
async fn create_person_born_in(client: &GrampsClient, year: i32) -> (Handle, Handle) {
    use gramps_web_mcp_rs::models::GrampsDate;

    let event_handle = create::create_event(
        client,
        CreateEventRequest {
            event_type: Some(serde_json::json!("Birth")),
            date: Some(GrampsDate {
                dateval: Some(serde_json::json!([1, 1, year, false])),
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .await
    .expect("creating the birth event fixture should succeed");

    let person_handle = create::create_person(
        client,
        CreatePersonRequest {
            event_ref_list: Some(vec![
                serde_json::json!({"ref": event_handle, "role": "Primary"}),
            ]),
            ..Default::default()
        },
    )
    .await
    .expect("creating the person fixture should succeed");

    // Adding to event_ref_list alone doesn't register it as *the* birth event — Gramps
    // Web's `sort=birth` reads `birth_ref_index` (an index into event_ref_list), which
    // stays -1 until set explicitly; it's the only event here, so that's index 0.
    update::update_person(
        client,
        &person_handle,
        &serde_json::json!({"birth_ref_index": 0}),
    )
    .await
    .expect("setting birth_ref_index on the person fixture should succeed");

    (person_handle, event_handle)
}

#[tokio::test]
async fn list_objects_sort_by_birth_ascending_and_descending() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let (older, older_event) = create_person_born_in(client, 1950).await;
    let (younger, younger_event) = create_person_born_in(client, 1980).await;

    let ascending = get::get_object_collection(
        client,
        "people",
        get::CollectionQuery {
            sort: Some(&["birth".to_string()]),
            ..Default::default()
        },
        get::ReadOptions::default(),
    )
    .await
    .unwrap();
    let ascending_handles: Vec<&str> = ascending
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|p| p["handle"].as_str())
        .filter(|h| *h == older || *h == younger)
        .collect();
    assert_eq!(
        ascending_handles,
        vec![older.as_str(), younger.as_str()],
        "sort=[\"birth\"] should order oldest first"
    );

    let descending = get::get_object_collection(
        client,
        "people",
        get::CollectionQuery {
            sort: Some(&["-birth".to_string()]),
            ..Default::default()
        },
        get::ReadOptions::default(),
    )
    .await
    .unwrap();
    let descending_handles: Vec<&str> = descending
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|p| p["handle"].as_str())
        .filter(|h| *h == older || *h == younger)
        .collect();
    assert_eq!(
        descending_handles,
        vec![younger.as_str(), older.as_str()],
        "sort=[\"-birth\"] should reverse the order"
    );

    delete::delete_object(client, "people", &older)
        .await
        .unwrap();
    delete::delete_object(client, "people", &younger)
        .await
        .unwrap();
    delete::delete_object(client, "events", &older_event)
        .await
        .unwrap();
    delete::delete_object(client, "events", &younger_event)
        .await
        .unwrap();
}

#[tokio::test]
async fn list_objects_sort_rules_and_pagesize_together() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let (p1, e1) = create_person_born_in(client, 1900).await;
    let (p2, e2) = create_person_born_in(client, 1910).await;

    let rules = matches_query_rule("exists(events)");
    let result = get::get_object_collection(
        client,
        "people",
        get::CollectionQuery {
            rules: Some(&rules),
            pagesize: Some(50),
            sort: Some(&["birth".to_string()]),
            ..Default::default()
        },
        get::ReadOptions::default(),
    )
    .await
    .unwrap();

    let handles: Vec<&str> = result
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|p| p["handle"].as_str())
        .filter(|h| *h == p1 || *h == p2)
        .collect();
    assert_eq!(
        handles,
        vec![p1.as_str(), p2.as_str()],
        "sort should still apply alongside rules + pagesize"
    );

    delete::delete_object(client, "people", &p1).await.unwrap();
    delete::delete_object(client, "people", &p2).await.unwrap();
    delete::delete_object(client, "events", &e1).await.unwrap();
    delete::delete_object(client, "events", &e2).await.unwrap();
}

#[tokio::test]
async fn list_objects_unknown_sort_key_reaches_gramps_web_and_gets_a_hint() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    // No local validation of `sort` keys anymore — the connector sends it through as-is and
    // lets Gramps Web itself reject what it doesn't recognize.
    let sort = vec!["not_a_real_sort_key".to_string()];
    let err = get::get_object_collection(
        client,
        "people",
        get::CollectionQuery {
            sort: Some(&sort),
            ..Default::default()
        },
        get::ReadOptions::default(),
    )
    .await
    .expect_err("Gramps Web should reject an unknown sort key itself");

    let hinted = get::hint_unknown_sort_key(err, Some(&sort));
    let msg = hinted.to_string();
    assert!(
        msg.contains("sort"),
        "error should hint that `sort` might be the cause, got: {msg}"
    );
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

    let obj = get::get_object_by_handle(client, "people", &handle, get::ReadOptions::default())
        .await
        .unwrap();
    let gramps_id = obj["gramps_id"].as_str().expect("gramps_id missing");

    let rules = matches_query_rule(&format!(r#"gramps_id == "{gramps_id}""#));
    let result = get::get_object_collection(
        client,
        "people",
        get::CollectionQuery {
            gramps_id: None,
            rules: Some(&rules),
            page: None,
            pagesize: None,
            handles: None,
            sort: None,
        },
        get::ReadOptions::default(),
    )
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
    let paged = get::get_object_collection(
        client,
        "people",
        get::CollectionQuery {
            gramps_id: None,
            rules: Some(&rules),
            page: Some(1),
            pagesize: Some(1),
            handles: None,
            sort: None,
        },
        get::ReadOptions::default(),
    )
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
    let paged = get::get_object_collection(
        client,
        "people",
        get::CollectionQuery {
            gramps_id: None,
            rules: Some(&rules),
            page: None,
            pagesize: Some(2),
            handles: None,
            sort: None,
        },
        get::ReadOptions::default(),
    )
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
    let result = get::get_object_collection(
        client,
        "families",
        get::CollectionQuery {
            gramps_id: None,
            rules: Some(&rules),
            page: None,
            pagesize: None,
            handles: None,
            sort: None,
        },
        get::ReadOptions::default(),
    )
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
