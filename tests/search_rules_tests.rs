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

//! `search`, `get_object`, `list_objects`, and `get_filter_rules` against a real Gramps Web.
//!
//! `keys`/`strip`/`profile` query-parameter *construction* is unit-tested on `ReadOptions` in
//! `src/tools/get.rs`; here it's enough to confirm once per path (get_object by handle,
//! list_objects) that Gramps Web actually honors them — see
//! `get_object_by_handle_respects_keys_strip_and_profile`/
//! `list_objects_respects_keys_strip_and_profile`. Filtering by `gramps_id` is similarly kept
//! to one test per tool (`rules_filter_people_by_gramps_id` for list_objects via `rules`,
//! `query_filter_people_by_gramps_id` in `query_tests.rs` for query_object), with pagination
//! covered separately in `list_objects_pagination_caps_results`.

#[path = "support/helpers.rs"]
mod common;
#[path = "support/mcp.rs"]
mod mcp;

use common::TestFixture;
use mcp::{expect_created_handle, expect_error_text, expect_ok, McpHarness};
use serde_json::json;

fn matches_query_rule(goql_expr: &str) -> serde_json::Value {
    json!({"rules": [{"name": "MatchesQuery", "values": [goql_expr]}]})
}

/// Creates a person with a birth event dated `year` so `sort=["birth"]`/`["-birth"]` has
/// something real to compare. Returns (person handle, event handle).
async fn create_person_born_in(harness: &McpHarness, year: i32) -> (String, String) {
    let event_handle = expect_created_handle(
        harness
            .call(
                "create_event",
                json!({"event_type": "Birth", "date": [1, 1, year]}),
            )
            .await,
    );
    let person_handle = expect_created_handle(harness.call("create_person", json!({})).await);
    // Gramps Web's `sort=birth` reads `birth_ref_index` (an index into event_ref_list), which
    // stays -1 until set explicitly; it's the only event here, so that's index 0.
    expect_ok(
        harness
            .call(
                "update_person",
                json!({
                    "handle": person_handle,
                    "data": {
                        "event_ref_list": [{"ref": event_handle, "role": "Primary"}],
                        "birth_ref_index": 0,
                    },
                }),
            )
            .await,
    );
    (person_handle, event_handle)
}

#[tokio::test]
async fn search_all_object_types_return_ok() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    expect_created_handle(
        harness
            .call("create_person", json!({"first_name": "Searchable"}))
            .await,
    );
    let source = expect_created_handle(
        harness
            .call("create_source", json!({"title": "Search Source"}))
            .await,
    );
    expect_created_handle(
        harness
            .call(
                "create_citation",
                json!({"source_handle": source, "page": "p. 1"}),
            )
            .await,
    );
    expect_created_handle(
        harness
            .call("create_event", json!({"event_type": "Birth"}))
            .await,
    );
    expect_created_handle(
        harness
            .call("create_place", json!({"title": "Search Place"}))
            .await,
    );
    expect_created_handle(harness.call("create_family", json!({})).await);
    expect_created_handle(
        harness
            .call("create_note", json!({"text": "search note text"}))
            .await,
    );
    expect_created_handle(
        harness
            .call("create_tag", json!({"name": "SearchTag"}))
            .await,
    );
    expect_created_handle(
        harness
            .call("create_repository", json!({"name": "Search Repo"}))
            .await,
    );
    expect_created_handle(
        harness
            .call("create_media", json!({"path": "/tmp/search.jpg"}))
            .await,
    );

    // Each type must be reachable and return a JSON array.
    // If a type is not yet indexed, the server returns Ok([]) — still valid.
    for (query, object_type) in [
        ("Searchable", Some("person")),
        ("Search Source", Some("source")),
        ("citation", Some("citation")),
        ("Birth", Some("event")),
        ("Search Place", Some("place")),
        ("family", Some("family")),
        ("note", Some("note")),
        ("SearchTag", Some("tag")),
        ("Search Repo", Some("repository")),
        ("search", Some("media")),
        ("Search", None),
    ] {
        let args = match object_type {
            Some(t) => json!({"query": query, "object_type": t}),
            None => json!({"query": query}),
        };
        let result = expect_ok(harness.call("search", args).await);
        assert!(
            result.is_array(),
            "search({query:?}, {object_type:?}) should return an array, got {result}"
        );
    }
}

#[tokio::test]
async fn get_object_by_gramps_id_returns_single_object_not_array() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    let handle = expect_created_handle(harness.call("create_person", json!({})).await);
    let obj = expect_ok(
        harness
            .call(
                "get_object",
                json!({"object_type": "person", "handle": handle}),
            )
            .await,
    );
    let gramps_id = obj["gramps_id"].as_str().expect("gramps_id missing");

    let by_id = expect_ok(
        harness
            .call(
                "get_object",
                json!({"object_type": "person", "gramps_id": gramps_id}),
            )
            .await,
    );
    assert!(
        by_id.is_object(),
        "get_object by gramps_id should return a single object, not an array; got {by_id}"
    );
    assert_eq!(by_id["handle"].as_str(), Some(handle.as_str()));
}

#[tokio::test]
async fn get_object_by_gramps_id_missing_returns_not_found() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    let error = expect_error_text(
        harness
            .call(
                "get_object",
                json!({"object_type": "person", "gramps_id": "NONEXISTENT_GRAMPS_ID"}),
            )
            .await,
    );
    assert!(error.contains("not found"), "got: {error}");
}

// The ambiguous-match (gramps_id matching >1 object) branch is covered by a unit test on
// `resolve_gramps_id_match` in src/tools/get.rs — Gramps Web enforces gramps_id uniqueness
// server-side, so a genuine collision isn't reproducible through the live API.

#[tokio::test]
async fn get_object_by_handle_respects_keys_strip_and_profile() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    let handle = expect_created_handle(
        harness
            .call(
                "create_person",
                json!({"first_name": "ProfileTest", "gender": 1}),
            )
            .await,
    );

    // keys: response is restricted to exactly the requested fields.
    let obj = expect_ok(
        harness
            .call(
                "get_object",
                json!({
                    "object_type": "person",
                    "handle": handle,
                    "keys": ["gramps_id", "primary_name"],
                }),
            )
            .await,
    );
    let fields: Vec<&str> = obj
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert!(
        fields
            .iter()
            .all(|f| *f == "gramps_id" || *f == "primary_name"),
        "response should only contain the requested keys, got {fields:?}"
    );

    // strip: on by default (no `strip` argument at all — the tools' own default, independent
    // of Gramps Web's own strip=false default), droppable with strip=false.
    let default_opts = expect_ok(
        harness
            .call(
                "get_object",
                json!({"object_type": "person", "handle": handle}),
            )
            .await,
    );
    assert!(
        default_opts.get("note_list").is_none(),
        "empty fields should be omitted by default (no `strip` argument at all), got {default_opts}"
    );
    let strip_false = expect_ok(
        harness
            .call(
                "get_object",
                json!({"object_type": "person", "handle": handle, "strip": false}),
            )
            .await,
    );
    assert_eq!(
        strip_false.get("note_list"),
        Some(&json!([])),
        "strip=false should opt back into the full object, including empty note_list"
    );
    assert_eq!(
        strip_false.get("private"),
        Some(&json!(false)),
        "private=false is falsy but not empty — must never be stripped, got {strip_false}"
    );

    // profile: adds a pre-formatted summary; strip=false here so the self-profile's own
    // empty birth/death summaries (no birth/death event on this person) aren't stripped too.
    let with_profile = expect_ok(
        harness
            .call(
                "get_object",
                json!({
                    "object_type": "person",
                    "handle": handle,
                    "profile": ["self"],
                    "strip": false,
                }),
            )
            .await,
    );
    let profile = with_profile
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
}

#[tokio::test]
async fn list_objects_respects_keys_strip_and_profile() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    let handle = expect_created_handle(
        harness
            .call("create_person", json!({"first_name": "ListProfileTest"}))
            .await,
    );
    let full = expect_ok(
        harness
            .call(
                "get_object",
                json!({"object_type": "person", "handle": handle}),
            )
            .await,
    );
    let gramps_id = full["gramps_id"].as_str().expect("gramps_id missing");
    let rules = matches_query_rule(&format!(r#"gramps_id == "{gramps_id}""#));

    // keys + strip together: every item restricted to the requested keys, and strip=true
    // still drops an empty field even though it was explicitly requested via keys.
    let result = expect_ok(
        harness
            .call(
                "list_objects",
                json!({
                    "object_type": "person",
                    "rules": rules,
                    "keys": ["gramps_id", "primary_name", "note_list"],
                    "strip": true,
                }),
            )
            .await,
    );
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
        fields
            .iter()
            .all(|f| *f == "gramps_id" || *f == "primary_name"),
        "every field should be one of the requested keys, got {fields:?}"
    );
    assert!(
        item.get("note_list").is_none(),
        "strip=true should drop the empty note_list even though it was requested via keys, got {item}"
    );

    // profile: same `rules` filter, this time asking for the self-profile summary.
    let result = expect_ok(
        harness
            .call(
                "list_objects",
                json!({"object_type": "person", "rules": rules, "profile": ["self"]}),
            )
            .await,
    );
    let items = result.as_array().expect("expected an array");
    assert_eq!(items.len(), 1);
    assert_eq!(
        items[0]["profile"]["name_given"].as_str(),
        Some("ListProfileTest"),
        "got {}",
        items[0]
    );
}

#[tokio::test]
async fn list_objects_with_handles_preserves_order_and_skips_unknown() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    let h1 = expect_created_handle(harness.call("create_person", json!({})).await);
    let h2 = expect_created_handle(harness.call("create_person", json!({})).await);
    let h3 = expect_created_handle(harness.call("create_person", json!({})).await);

    // Deliberately out of creation order, plus one handle that doesn't exist.
    let result = expect_ok(
        harness
            .call(
                "list_objects",
                json!({
                    "object_type": "person",
                    "handles": [h3, h1, "NONEXISTENT_HANDLE", h2],
                }),
            )
            .await,
    );

    let items = result.as_array().expect("expected an array");
    let returned_handles: Vec<&str> = items
        .iter()
        .map(|i| i["handle"].as_str().expect("handle missing"))
        .collect();
    assert_eq!(
        returned_handles,
        vec![h3.as_str(), h1.as_str(), h2.as_str()],
        "handles should come back in the requested order, and the unknown one skipped rather \
         than erroring"
    );
}

#[tokio::test]
async fn list_objects_sort_by_birth_ascending_and_descending() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    let (older, _older_event) = create_person_born_in(&harness, 1950).await;
    let (younger, _younger_event) = create_person_born_in(&harness, 1980).await;

    let ascending = expect_ok(
        harness
            .call(
                "list_objects",
                json!({"object_type": "person", "sort": ["birth"]}),
            )
            .await,
    );
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

    let descending = expect_ok(
        harness
            .call(
                "list_objects",
                json!({"object_type": "person", "sort": ["-birth"]}),
            )
            .await,
    );
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
}

#[tokio::test]
async fn list_objects_sort_rules_and_pagesize_together() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    let (p1, _e1) = create_person_born_in(&harness, 1900).await;
    let (p2, _e2) = create_person_born_in(&harness, 1910).await;

    let rules = matches_query_rule("exists(events)");
    let result = expect_ok(
        harness
            .call(
                "list_objects",
                json!({
                    "object_type": "person",
                    "rules": rules,
                    "pagesize": 50,
                    "sort": ["birth"],
                }),
            )
            .await,
    );

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
}

#[tokio::test]
async fn list_objects_unknown_sort_key_reaches_gramps_web_and_gets_a_hint() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    // No local validation of `sort` keys — the connector sends it through as-is and lets
    // Gramps Web itself reject what it doesn't recognize.
    let error = expect_error_text(
        harness
            .call(
                "list_objects",
                json!({"object_type": "person", "sort": ["not_a_real_sort_key"]}),
            )
            .await,
    );
    assert!(
        error.contains("sort"),
        "error should hint that `sort` might be the cause, got: {error}"
    );
}

#[tokio::test]
async fn list_objects_pagination_caps_results() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    // Plain pagination, no filter: page=1 pagesize=1 must return at most 1 item.
    expect_created_handle(harness.call("create_person", json!({})).await);
    expect_created_handle(harness.call("create_person", json!({})).await);
    let page1 = expect_ok(
        harness
            .call(
                "list_objects",
                json!({"object_type": "person", "page": 1, "pagesize": 1}),
            )
            .await,
    );
    assert!(
        page1.as_array().unwrap().len() <= 1,
        "pagesize=1 should return at most 1 item"
    );

    // Each person heads their own family (count(families) > 0) and has no parent family
    // (not exists(parent_families)) — more matches than the pagesize below.
    for _ in 0..4 {
        let person = expect_created_handle(harness.call("create_person", json!({})).await);
        expect_created_handle(
            harness
                .call("create_family", json!({"father_handle": person}))
                .await,
        );
    }
    let rules = matches_query_rule("count(families) > 0 and not exists(parent_families)");

    // rules + explicit page + pagesize.
    let paged = expect_ok(
        harness
            .call(
                "list_objects",
                json!({
                    "object_type": "person",
                    "rules": rules,
                    "page": 1,
                    "pagesize": 1,
                }),
            )
            .await,
    );
    assert!(
        paged.as_array().unwrap().len() <= 1,
        "rules + pagesize=1 should cap results"
    );

    // Gramps Web defaults `page` to 0 when omitted, which disables pagination and silently
    // ignores `pagesize` — pagesize alone, with no page, must still cap.
    let paged_no_page = expect_ok(
        harness
            .call(
                "list_objects",
                json!({"object_type": "person", "rules": rules, "pagesize": 2}),
            )
            .await,
    );
    assert!(
        paged_no_page.as_array().unwrap().len() <= 2,
        "pagesize=2 without page should still cap results, got {paged_no_page}"
    );
}

#[tokio::test]
async fn search_pagination() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    // Create two people with the same distinctive name so search finds at least 2 results.
    expect_created_handle(
        harness
            .call("create_person", json!({"first_name": "Pagination"}))
            .await,
    );
    expect_created_handle(
        harness
            .call("create_person", json!({"first_name": "Pagination"}))
            .await,
    );

    // pagesize=1 must return at most 1 result
    let page1 = expect_ok(
        harness
            .call(
                "search",
                json!({"query": "Pagination", "object_type": "person", "page": 1, "pagesize": 1}),
            )
            .await,
    );
    assert!(
        page1.as_array().unwrap().len() <= 1,
        "pagesize=1 should return at most 1 result, got {}",
        page1.as_array().unwrap().len()
    );

    // page=2 pagesize=1 — second page should also be an array (may be empty if not yet indexed)
    let page2 = expect_ok(
        harness
            .call(
                "search",
                json!({"query": "Pagination", "object_type": "person", "page": 2, "pagesize": 1}),
            )
            .await,
    );
    assert!(page2.is_array(), "page 2 should return an array");

    // No pagination params — should also be fine (backward compat)
    let all = expect_ok(
        harness
            .call(
                "search",
                json!({"query": "Pagination", "object_type": "person"}),
            )
            .await,
    );
    assert!(
        all.is_array(),
        "search without pagination should return an array"
    );
}

#[tokio::test]
async fn rules_filter_people_by_gramps_id() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    let handle = expect_created_handle(harness.call("create_person", json!({})).await);
    let obj = expect_ok(
        harness
            .call(
                "get_object",
                json!({"object_type": "person", "handle": handle}),
            )
            .await,
    );
    let gramps_id = obj["gramps_id"].as_str().expect("gramps_id missing");

    let rules = matches_query_rule(&format!(r#"gramps_id == "{gramps_id}""#));
    let result = expect_ok(
        harness
            .call(
                "list_objects",
                json!({"object_type": "person", "rules": rules}),
            )
            .await,
    );

    assert!(result.is_array(), "rules filter should return an array");
    let items = result.as_array().unwrap();
    assert!(
        items
            .iter()
            .any(|p| p["handle"].as_str() == Some(handle.as_str())),
        "rules filter should find the created person"
    );
}

#[tokio::test]
async fn rules_filter_families_by_child_count() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    let child = expect_created_handle(harness.call("create_person", json!({})).await);
    let empty_family = expect_created_handle(harness.call("create_family", json!({})).await);
    let family_with_child = expect_created_handle(harness.call("create_family", json!({})).await);
    expect_ok(
        harness
            .call(
                "update_family",
                json!({
                    "handle": family_with_child,
                    "data": {"child_ref_list": [{"ref": child}]},
                }),
            )
            .await,
    );

    let rules = matches_query_rule("count(children) > 0");
    let result = expect_ok(
        harness
            .call(
                "list_objects",
                json!({"object_type": "family", "rules": rules}),
            )
            .await,
    );

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
}

#[tokio::test]
async fn get_filter_rules_lists_matches_query() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    let result = expect_ok(
        harness
            .call("get_filter_rules", json!({"object_type": "person"}))
            .await,
    );

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
