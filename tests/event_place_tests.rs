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

#[tokio::test]
async fn event_round_trip() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    let handle = expect_created_handle(
        harness
            .call(
                "create_event",
                json!({"event_type": "Birth", "description": "Test birth event"}),
            )
            .await,
    );

    let event = expect_ok(
        harness
            .call(
                "get_object",
                json!({"object_type": "event", "handle": handle}),
            )
            .await,
    );
    assert_eq!(event["description"].as_str(), Some("Test birth event"));
    assert!(!event["type"].is_null());

    expect_ok(
        harness
            .call(
                "update_event",
                json!({"handle": handle, "data": {"description": "Updated description"}}),
            )
            .await,
    );
    let updated = expect_ok(
        harness
            .call(
                "get_object",
                json!({"object_type": "event", "handle": handle}),
            )
            .await,
    );
    assert_eq!(updated["description"].as_str(), Some("Updated description"));

    expect_success(
        harness
            .call(
                "delete_object",
                json!({"object_type": "event", "handle": handle}),
            )
            .await,
    );
    let error = expect_error_text(
        harness
            .call(
                "get_object",
                json!({"object_type": "event", "handle": handle}),
            )
            .await,
    );
    assert!(error.contains("not found"), "got: {error}");
}

#[tokio::test]
async fn place_round_trip() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    let handle = expect_created_handle(
        harness
            .call("create_place", json!({"title": "Moscow"}))
            .await,
    );

    let place = expect_ok(
        harness
            .call(
                "get_object",
                json!({"object_type": "place", "handle": handle}),
            )
            .await,
    );
    assert_eq!(place["title"].as_str(), Some("Moscow"));

    expect_ok(
        harness
            .call(
                "update_place",
                json!({"handle": handle, "data": {"title": "Saint Petersburg"}}),
            )
            .await,
    );
    let updated = expect_ok(
        harness
            .call(
                "get_object",
                json!({"object_type": "place", "handle": handle}),
            )
            .await,
    );
    assert_eq!(updated["title"].as_str(), Some("Saint Petersburg"));

    expect_success(
        harness
            .call(
                "delete_object",
                json!({"object_type": "place", "handle": handle}),
            )
            .await,
    );
    let error = expect_error_text(
        harness
            .call(
                "get_object",
                json!({"object_type": "place", "handle": handle}),
            )
            .await,
    );
    assert!(error.contains("not found"), "got: {error}");
}

#[tokio::test]
async fn event_span_between_two_events() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    let handle1 = expect_created_handle(
        harness
            .call("create_event", json!({"event_type": "Birth"}))
            .await,
    );
    let handle2 = expect_created_handle(
        harness
            .call("create_event", json!({"event_type": "Death"}))
            .await,
    );

    expect_ok(
        harness
            .call(
                "get_event_span",
                json!({"handle1": handle1, "handle2": handle2}),
            )
            .await,
    );
}
