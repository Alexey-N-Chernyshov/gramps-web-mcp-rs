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
#[path = "support/mcp.rs"]
mod mcp;

use common::TestFixture;
use mcp::{expect_created_handle, expect_error_text, expect_ok, expect_success, McpHarness};
use serde_json::json;

#[tokio::test]
async fn source_round_trip() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    let handle = expect_created_handle(
        harness
            .call(
                "create_source",
                json!({"title": "Vital Records 1850", "author": "County Office"}),
            )
            .await,
    );

    let source = expect_ok(
        harness
            .call(
                "get_object",
                json!({"object_type": "source", "handle": handle}),
            )
            .await,
    );
    assert_eq!(source["title"].as_str(), Some("Vital Records 1850"));
    assert_eq!(source["author"].as_str(), Some("County Office"));

    expect_ok(
        harness
            .call(
                "update_source",
                json!({"handle": handle, "data": {"title": "Vital Records 1900"}}),
            )
            .await,
    );
    let updated = expect_ok(
        harness
            .call(
                "get_object",
                json!({"object_type": "source", "handle": handle}),
            )
            .await,
    );
    assert_eq!(updated["title"].as_str(), Some("Vital Records 1900"));

    expect_success(
        harness
            .call(
                "delete_object",
                json!({"object_type": "source", "handle": handle}),
            )
            .await,
    );
    let error = expect_error_text(
        harness
            .call(
                "get_object",
                json!({"object_type": "source", "handle": handle}),
            )
            .await,
    );
    assert!(error.contains("not found"), "got: {error}");
}

#[tokio::test]
async fn citation_links_source() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    let source_handle = expect_created_handle(
        harness
            .call("create_source", json!({"title": "Parish Records"}))
            .await,
    );

    let citation_handle = expect_created_handle(
        harness
            .call(
                "create_citation",
                json!({"source_handle": source_handle, "page": "p. 42"}),
            )
            .await,
    );

    let citation = expect_ok(
        harness
            .call(
                "get_object",
                json!({"object_type": "citation", "handle": citation_handle}),
            )
            .await,
    );
    assert_eq!(
        citation["source_handle"].as_str(),
        Some(source_handle.as_str())
    );
    assert_eq!(citation["page"].as_str(), Some("p. 42"));

    expect_ok(
        harness
            .call(
                "update_citation",
                json!({"handle": citation_handle, "data": {"page": "p. 99"}}),
            )
            .await,
    );
    let updated = expect_ok(
        harness
            .call(
                "get_object",
                json!({"object_type": "citation", "handle": citation_handle}),
            )
            .await,
    );
    assert_eq!(updated["page"].as_str(), Some("p. 99"));
}

#[tokio::test]
async fn note_round_trip() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    let handle = expect_created_handle(
        harness
            .call(
                "create_note",
                json!({"text": "Hello from test", "note_type": "General"}),
            )
            .await,
    );

    let note = expect_ok(
        harness
            .call(
                "get_object",
                json!({"object_type": "note", "handle": handle}),
            )
            .await,
    );
    assert_eq!(note["text"]["string"].as_str(), Some("Hello from test"));

    expect_ok(
        harness
            .call(
                "update_note",
                json!({"handle": handle, "data": {"text": {"string": "Updated note text"}}}),
            )
            .await,
    );
    let updated = expect_ok(
        harness
            .call(
                "get_object",
                json!({"object_type": "note", "handle": handle}),
            )
            .await,
    );
    assert_eq!(
        updated["text"]["string"].as_str(),
        Some("Updated note text")
    );

    expect_success(
        harness
            .call(
                "delete_object",
                json!({"object_type": "note", "handle": handle}),
            )
            .await,
    );
    let error = expect_error_text(
        harness
            .call(
                "get_object",
                json!({"object_type": "note", "handle": handle}),
            )
            .await,
    );
    assert!(error.contains("not found"), "got: {error}");
}

#[tokio::test]
async fn tag_round_trip() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    let handle = expect_created_handle(
        harness
            .call(
                "create_tag",
                json!({"name": "Important", "color": "#FF0000", "priority": 1}),
            )
            .await,
    );

    let tag = expect_ok(
        harness
            .call(
                "get_object",
                json!({"object_type": "tag", "handle": handle}),
            )
            .await,
    );
    assert_eq!(tag["name"].as_str(), Some("Important"));
    assert_eq!(tag["color"].as_str(), Some("#FF0000"));

    expect_ok(
        harness
            .call(
                "update_tag",
                json!({"handle": handle, "data": {"color": "#00FF00"}}),
            )
            .await,
    );
    let updated = expect_ok(
        harness
            .call(
                "get_object",
                json!({"object_type": "tag", "handle": handle}),
            )
            .await,
    );
    assert_eq!(updated["color"].as_str(), Some("#00FF00"));

    expect_success(
        harness
            .call(
                "delete_object",
                json!({"object_type": "tag", "handle": handle}),
            )
            .await,
    );
    let error = expect_error_text(
        harness
            .call(
                "get_object",
                json!({"object_type": "tag", "handle": handle}),
            )
            .await,
    );
    assert!(error.contains("not found"), "got: {error}");
}

#[tokio::test]
async fn repository_round_trip() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    let handle = expect_created_handle(
        harness
            .call(
                "create_repository",
                json!({"name": "National Archives", "repo_type": "Archive"}),
            )
            .await,
    );

    let repo = expect_ok(
        harness
            .call(
                "get_object",
                json!({"object_type": "repository", "handle": handle}),
            )
            .await,
    );
    assert_eq!(repo["name"].as_str(), Some("National Archives"));
    assert_eq!(repo["type"].as_str(), Some("Archive"));

    expect_ok(
        harness
            .call(
                "update_repository",
                json!({"handle": handle, "data": {"name": "State Archives"}}),
            )
            .await,
    );
    let updated = expect_ok(
        harness
            .call(
                "get_object",
                json!({"object_type": "repository", "handle": handle}),
            )
            .await,
    );
    assert_eq!(updated["name"].as_str(), Some("State Archives"));

    expect_success(
        harness
            .call(
                "delete_object",
                json!({"object_type": "repository", "handle": handle}),
            )
            .await,
    );
    let error = expect_error_text(
        harness
            .call(
                "get_object",
                json!({"object_type": "repository", "handle": handle}),
            )
            .await,
    );
    assert!(error.contains("not found"), "got: {error}");
}
