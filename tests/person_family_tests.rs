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
#[path = "support/mcp.rs"]
mod mcp;

use common::TestFixture;
use mcp::{expect_created_handle, expect_error_text, expect_ok, expect_success, McpHarness};
use serde_json::json;

/// Create → read → update → delete → confirm gone, for a person. (`create_and_get_person_round_trip`
/// and `person_lifecycle` used to test this as two separate tests; one scenario, one test.)
#[tokio::test]
async fn person_round_trip() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    let handle = expect_created_handle(
        harness
            .call(
                "create_person",
                json!({"first_name": "Ivan", "surname": "Petrov", "gender": 1}),
            )
            .await,
    );

    let person = expect_ok(
        harness
            .call(
                "get_object",
                json!({"object_type": "person", "handle": handle}),
            )
            .await,
    );
    assert_eq!(person["handle"].as_str(), Some(handle.as_str()));
    assert_eq!(person["primary_name"]["first_name"].as_str(), Some("Ivan"));

    // primary_name is a nested object: `data` is a JSON Merge Patch (RFC 7396), so a
    // partial patch here merges into the existing primary_name field-by-field instead of
    // replacing it wholesale.
    expect_ok(
        harness
            .call(
                "update_person",
                json!({"handle": handle, "data": {"primary_name": {"first_name": "Petr"}}}),
            )
            .await,
    );
    let updated = expect_ok(
        harness
            .call(
                "get_object",
                json!({"object_type": "person", "handle": handle}),
            )
            .await,
    );
    assert_eq!(updated["primary_name"]["first_name"].as_str(), Some("Petr"));
    assert_eq!(
        updated["primary_name"]["surname_list"][0]["surname"].as_str(),
        Some("Petrov"),
        "surname must survive the update, not be wiped out by the merge patch"
    );
    assert_eq!(
        updated["primary_name"]["type"].as_str(),
        Some("Birth Name"),
        "name type must survive the update, not be wiped out by the merge patch"
    );

    expect_success(
        harness
            .call(
                "delete_object",
                json!({"object_type": "person", "handle": handle}),
            )
            .await,
    );
    let error = expect_error_text(
        harness
            .call(
                "get_object",
                json!({"object_type": "person", "handle": handle}),
            )
            .await,
    );
    assert!(error.contains("not found"), "got: {error}");
}

#[tokio::test]
async fn family_with_parents() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    let father = expect_created_handle(harness.call("create_person", json!({"gender": 1})).await);
    let mother = expect_created_handle(harness.call("create_person", json!({"gender": 2})).await);

    let family_handle = expect_created_handle(
        harness
            .call(
                "create_family",
                json!({"father_handle": father, "mother_handle": mother}),
            )
            .await,
    );

    let family = expect_ok(
        harness
            .call(
                "get_object",
                json!({"object_type": "family", "handle": family_handle}),
            )
            .await,
    );
    assert_eq!(family["father_handle"].as_str(), Some(father.as_str()));
    assert_eq!(family["mother_handle"].as_str(), Some(mother.as_str()));

    let new_mother =
        expect_created_handle(harness.call("create_person", json!({"gender": 2})).await);
    expect_ok(
        harness
            .call(
                "update_family",
                json!({"handle": family_handle, "data": {"mother_handle": new_mother}}),
            )
            .await,
    );
    let updated = expect_ok(
        harness
            .call(
                "get_object",
                json!({"object_type": "family", "handle": family_handle}),
            )
            .await,
    );
    assert_eq!(
        updated["mother_handle"].as_str(),
        Some(new_mother.as_str()),
        "mother_handle should be updated"
    );
}

#[tokio::test]
async fn family_with_child() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    let child = expect_created_handle(harness.call("create_person", json!({})).await);
    let family_handle = expect_created_handle(harness.call("create_family", json!({})).await);

    // create_family doesn't expose child_ref_list directly — set it via update_family, same
    // as any other field a caller wants beyond the minimal creation shape.
    expect_ok(
        harness
            .call(
                "update_family",
                json!({
                    "handle": family_handle,
                    "data": {"child_ref_list": [{"ref": child}]},
                }),
            )
            .await,
    );

    let family = expect_ok(
        harness
            .call(
                "get_object",
                json!({"object_type": "family", "handle": family_handle}),
            )
            .await,
    );
    let children = family["child_ref_list"].as_array().unwrap();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0]["ref"].as_str(), Some(child.as_str()));
}

#[tokio::test]
async fn person_timeline() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    let handle = expect_created_handle(harness.call("create_person", json!({})).await);

    let timeline = expect_ok(
        harness
            .call("get_person_timeline", json!({"handle": handle}))
            .await,
    );
    assert!(timeline.is_array(), "timeline should be an array");
}

#[tokio::test]
async fn family_timeline() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    let family_handle = expect_created_handle(harness.call("create_family", json!({})).await);

    let timeline = expect_ok(
        harness
            .call("get_family_timeline", json!({"handle": family_handle}))
            .await,
    );
    assert!(timeline.is_array(), "timeline should be an array");
}

#[tokio::test]
async fn relations_between_parent_and_child() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    let father = expect_created_handle(harness.call("create_person", json!({"gender": 1})).await);
    let child = expect_created_handle(harness.call("create_person", json!({})).await);

    // Attach the child the same way `family_with_child` does.
    let family_handle = expect_created_handle(
        harness
            .call("create_family", json!({"father_handle": father}))
            .await,
    );
    expect_ok(
        harness
            .call(
                "update_family",
                json!({
                    "handle": family_handle,
                    "data": {"child_ref_list": [{"ref": child}]},
                }),
            )
            .await,
    );

    expect_ok(
        harness
            .call(
                "get_relations",
                json!({"handle1": father, "handle2": child}),
            )
            .await,
    );
}

#[tokio::test]
async fn partial_update_preserves_unspecified_person_fields() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    let note_handle = expect_created_handle(
        harness
            .call("create_note", json!({"text": "A research note"}))
            .await,
    );
    let source_handle = expect_created_handle(
        harness
            .call("create_source", json!({"title": "Census 1900"}))
            .await,
    );
    let citation_handle = expect_created_handle(
        harness
            .call(
                "create_citation",
                json!({"source_handle": source_handle, "page": "p. 12"}),
            )
            .await,
    );
    let event_handle = expect_created_handle(
        harness
            .call("create_event", json!({"event_type": "Birth"}))
            .await,
    );

    let handle = expect_created_handle(harness.call("create_person", json!({"gender": 1})).await);
    // create_person only takes first_name/surname/gender — set the richer lists via
    // update_person, same mechanism the partial update below is about to exercise.
    expect_ok(
        harness
            .call(
                "update_person",
                json!({
                    "handle": handle,
                    "data": {
                        "note_list": [note_handle],
                        "citation_list": [citation_handle],
                        "event_ref_list": [{"ref": event_handle, "role": "Primary"}],
                    },
                }),
            )
            .await,
    );

    // A partial update touching only `gender` must leave the other lists untouched.
    expect_ok(
        harness
            .call(
                "update_person",
                json!({"handle": handle, "data": {"gender": 2}}),
            )
            .await,
    );

    let updated = expect_ok(
        harness
            .call(
                "get_object",
                json!({"object_type": "person", "handle": handle}),
            )
            .await,
    );
    assert_eq!(updated["gender"].as_i64(), Some(2));
    assert_eq!(
        updated["note_list"].as_array().unwrap(),
        &vec![json!(note_handle)]
    );
    assert_eq!(
        updated["citation_list"].as_array().unwrap(),
        &vec![json!(citation_handle)]
    );
    let event_refs = updated["event_ref_list"].as_array().unwrap();
    assert_eq!(event_refs.len(), 1);
    assert_eq!(event_refs[0]["ref"].as_str(), Some(event_handle.as_str()));
}

#[tokio::test]
async fn partial_update_replaces_list_when_included_in_data() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    let note1 = expect_created_handle(
        harness
            .call("create_note", json!({"text": "First note"}))
            .await,
    );
    let note2 = expect_created_handle(
        harness
            .call("create_note", json!({"text": "Second note"}))
            .await,
    );

    let handle = expect_created_handle(harness.call("create_person", json!({})).await);
    expect_ok(
        harness
            .call(
                "update_person",
                json!({"handle": handle, "data": {"note_list": [note1]}}),
            )
            .await,
    );

    expect_ok(
        harness
            .call(
                "update_person",
                json!({"handle": handle, "data": {"note_list": [note2]}}),
            )
            .await,
    );

    let updated = expect_ok(
        harness
            .call(
                "get_object",
                json!({"object_type": "person", "handle": handle}),
            )
            .await,
    );
    assert_eq!(
        updated["note_list"].as_array().unwrap(),
        &vec![json!(note2)],
        "note_list should be replaced wholesale, not merged with the old one"
    );
}

#[tokio::test]
async fn update_person_merge_patch_preserves_other_nested_fields() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    let handle = expect_created_handle(
        harness
            .call(
                "create_person",
                json!({"first_name": "Original", "surname": "Smith"}),
            )
            .await,
    );
    // primary_name now has filled subfields (first_name, surname_list, type) but
    // create_person doesn't expose nick/call — add those with a minimal merge patch
    // (RFC 7396): it merges into the existing primary_name field-by-field, so there's no
    // need to read the object first and round-trip its other subfields back.
    expect_ok(
        harness
            .call(
                "update_person",
                json!({"handle": handle, "data": {"primary_name": {"nick": "", "call": ""}}}),
            )
            .await,
    );

    // Read with the tools' default (strip=true): strip recurses into nested objects, so
    // the empty primary_name.nick/call subfields are dropped here, not just empty
    // top-level fields.
    let stripped = expect_ok(
        harness
            .call(
                "get_object",
                json!({"object_type": "person", "handle": handle}),
            )
            .await,
    );
    let primary_name = &stripped["primary_name"];
    assert!(
        primary_name.get("nick").is_none() && primary_name.get("call").is_none(),
        "sanity check: strip should have dropped the empty nick/call subfields, got {primary_name}"
    );

    // Patch just first_name — a merge patch, so this must not disturb nick/call even
    // though the stripped read above never saw them.
    expect_ok(
        harness
            .call(
                "update_person",
                json!({"handle": handle, "data": {"primary_name": {"first_name": "Updated"}}}),
            )
            .await,
    );

    // Read back the full object (strip=false) to see exactly what landed.
    let full = expect_ok(
        harness
            .call(
                "get_object",
                json!({"object_type": "person", "handle": handle, "strip": false}),
            )
            .await,
    );
    let full_name = &full["primary_name"];
    assert_eq!(
        full_name["first_name"].as_str(),
        Some("Updated"),
        "got {full_name}"
    );
    assert_eq!(
        full_name["surname_list"][0]["surname"].as_str(),
        Some("Smith"),
        "surname_list should survive the round trip, got {full_name}"
    );
    assert_eq!(
        full_name["type"].as_str(),
        Some("Birth Name"),
        "got {full_name}"
    );
    assert_eq!(
        full_name["nick"].as_str(),
        Some(""),
        "nick should still be present and empty, not dropped by the round trip, got {full_name}"
    );
    assert_eq!(
        full_name["call"].as_str(),
        Some(""),
        "call should still be present and empty, not dropped by the round trip, got {full_name}"
    );
}

/// `update_person` with a `data.handle` that doesn't match the `handle` argument is already
/// confirmed to be *rejected* with a clear error in `mcp_server_tests.rs`
/// (`update_person_rejects_mismatched_handle_in_data`) — that doesn't need a real Gramps Web.
/// What does need one is confirming the rejected write didn't partially apply, which is what
/// this test is for.
#[tokio::test]
async fn update_person_rejects_mismatched_handle_without_writing() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    let handle = expect_created_handle(harness.call("create_person", json!({"gender": 1})).await);

    expect_error_text(
        harness
            .call(
                "update_person",
                json!({"handle": handle, "data": {"handle": "some-other-handle", "gender": 2}}),
            )
            .await,
    );

    let unchanged = expect_ok(
        harness
            .call(
                "get_object",
                json!({"object_type": "person", "handle": handle}),
            )
            .await,
    );
    assert_eq!(
        unchanged["gender"].as_i64(),
        Some(1),
        "object must be left untouched when the write is rejected"
    );
}
