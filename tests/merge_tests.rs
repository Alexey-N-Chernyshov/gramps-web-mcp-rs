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
use mcp::{expect_created_handle, expect_error_text, expect_success, McpHarness};
use serde_json::json;

/// Exercises every `merge_*` tool in one pass: create two objects of a type, merge them, and
/// confirm the duplicate is gone (survivor's handle is left in place by construction, so no
/// `get_object` round trip is needed on the happy path itself).
#[tokio::test]
async fn merge_operations() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    // merge_person
    let p1 = expect_created_handle(harness.call("create_person", json!({})).await);
    let p2 = expect_created_handle(harness.call("create_person", json!({})).await);
    expect_success(
        harness
            .call(
                "merge_person",
                json!({"survivor_handle": p1, "duplicate_handle": p2, "family_merger": false}),
            )
            .await,
    );
    let error = expect_error_text(
        harness
            .call("get_object", json!({"object_type": "person", "handle": p2}))
            .await,
    );
    assert!(error.contains("not found"), "got: {error}");

    // merge_family
    let f1 = expect_created_handle(harness.call("create_family", json!({})).await);
    let f2 = expect_created_handle(harness.call("create_family", json!({})).await);
    expect_success(
        harness
            .call(
                "merge_family",
                json!({"survivor_handle": f1, "duplicate_handle": f2}),
            )
            .await,
    );
    let error = expect_error_text(
        harness
            .call("get_object", json!({"object_type": "family", "handle": f2}))
            .await,
    );
    assert!(error.contains("not found"), "got: {error}");

    // merge_event
    let e1 = expect_created_handle(
        harness
            .call("create_event", json!({"event_type": "Birth"}))
            .await,
    );
    let e2 = expect_created_handle(
        harness
            .call("create_event", json!({"event_type": "Birth"}))
            .await,
    );
    expect_success(
        harness
            .call(
                "merge_event",
                json!({"survivor_handle": e1, "duplicate_handle": e2}),
            )
            .await,
    );
    let error = expect_error_text(
        harness
            .call("get_object", json!({"object_type": "event", "handle": e2}))
            .await,
    );
    assert!(error.contains("not found"), "got: {error}");

    // merge_place
    let pl1 = expect_created_handle(harness.call("create_place", json!({"title": "A"})).await);
    let pl2 = expect_created_handle(harness.call("create_place", json!({"title": "B"})).await);
    expect_success(
        harness
            .call(
                "merge_place",
                json!({"survivor_handle": pl1, "duplicate_handle": pl2}),
            )
            .await,
    );
    let error = expect_error_text(
        harness
            .call("get_object", json!({"object_type": "place", "handle": pl2}))
            .await,
    );
    assert!(error.contains("not found"), "got: {error}");

    // merge_note
    let n1 = expect_created_handle(
        harness
            .call("create_note", json!({"text": "note one"}))
            .await,
    );
    let n2 = expect_created_handle(
        harness
            .call("create_note", json!({"text": "note two"}))
            .await,
    );
    expect_success(
        harness
            .call(
                "merge_note",
                json!({"survivor_handle": n1, "duplicate_handle": n2}),
            )
            .await,
    );
    let error = expect_error_text(
        harness
            .call("get_object", json!({"object_type": "note", "handle": n2}))
            .await,
    );
    assert!(error.contains("not found"), "got: {error}");

    // merge_source + merge_citation + merge_repository
    let src1 = expect_created_handle(
        harness
            .call("create_source", json!({"title": "Source One"}))
            .await,
    );
    let src2 = expect_created_handle(
        harness
            .call("create_source", json!({"title": "Source Two"}))
            .await,
    );

    let c1 = expect_created_handle(
        harness
            .call("create_citation", json!({"source_handle": src1}))
            .await,
    );
    let c2 = expect_created_handle(
        harness
            .call("create_citation", json!({"source_handle": src2}))
            .await,
    );
    expect_success(
        harness
            .call(
                "merge_citation",
                json!({"survivor_handle": c1, "duplicate_handle": c2}),
            )
            .await,
    );
    let error = expect_error_text(
        harness
            .call(
                "get_object",
                json!({"object_type": "citation", "handle": c2}),
            )
            .await,
    );
    assert!(error.contains("not found"), "got: {error}");

    expect_success(
        harness
            .call(
                "merge_source",
                json!({"survivor_handle": src1, "duplicate_handle": src2}),
            )
            .await,
    );
    let error = expect_error_text(
        harness
            .call(
                "get_object",
                json!({"object_type": "source", "handle": src2}),
            )
            .await,
    );
    assert!(error.contains("not found"), "got: {error}");

    let r1 = expect_created_handle(
        harness
            .call("create_repository", json!({"name": "Repo One"}))
            .await,
    );
    let r2 = expect_created_handle(
        harness
            .call("create_repository", json!({"name": "Repo Two"}))
            .await,
    );
    expect_success(
        harness
            .call(
                "merge_repository",
                json!({"survivor_handle": r1, "duplicate_handle": r2}),
            )
            .await,
    );
    let error = expect_error_text(
        harness
            .call(
                "get_object",
                json!({"object_type": "repository", "handle": r2}),
            )
            .await,
    );
    assert!(error.contains("not found"), "got: {error}");

    // merge_media
    let m1 = expect_created_handle(
        harness
            .call("create_media", json!({"path": "/tmp/a.jpg"}))
            .await,
    );
    let m2 = expect_created_handle(
        harness
            .call("create_media", json!({"path": "/tmp/b.jpg"}))
            .await,
    );
    expect_success(
        harness
            .call(
                "merge_media",
                json!({"survivor_handle": m1, "duplicate_handle": m2}),
            )
            .await,
    );
    let error = expect_error_text(
        harness
            .call("get_object", json!({"object_type": "media", "handle": m2}))
            .await,
    );
    assert!(error.contains("not found"), "got: {error}");
}
