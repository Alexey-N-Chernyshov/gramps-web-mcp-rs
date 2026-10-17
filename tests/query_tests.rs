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
use mcp::{expect_created_handle, expect_ok, McpHarness};
use serde_json::json;

#[tokio::test]
async fn query_filter_people_by_gramps_id() {
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

    let result = expect_ok(
        harness
            .call(
                "query_object",
                json!({
                    "object_type": "person",
                    "where_expr": format!(r#"gramps_id == "{gramps_id}""#),
                }),
            )
            .await,
    );

    let items = result["items"]
        .as_array()
        .expect("query_object response should have an `items` array");
    assert!(
        items
            .iter()
            .any(|p| p["handle"].as_str() == Some(handle.as_str())),
        "where_expr filter should find the created person"
    );
}

#[tokio::test]
async fn query_limit_caps_results() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    harness
        .call("create_person", json!({"first_name": "QueryLimitOne"}))
        .await
        .expect("call_tool should succeed at the protocol level");
    harness
        .call("create_person", json!({"first_name": "QueryLimitTwo"}))
        .await
        .expect("call_tool should succeed at the protocol level");

    let result = expect_ok(
        harness
            .call("query_object", json!({"object_type": "person", "limit": 1}))
            .await,
    );

    let items = result["items"]
        .as_array()
        .expect("query_object response should have an `items` array");
    assert!(items.len() <= 1, "limit=1 should cap results to at most 1");
}

#[tokio::test]
async fn query_select_narrows_columns() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    let handle = expect_created_handle(harness.call("create_person", json!({})).await);

    let result = expect_ok(
        harness
            .call(
                "query_object",
                json!({
                    "object_type": "person",
                    "select": ["handle", "gramps_id"],
                    "where_expr": format!(r#"handle == "{handle}""#),
                }),
            )
            .await,
    );

    let items = result["items"]
        .as_array()
        .expect("query_object response should have an `items` array");
    assert_eq!(
        items.len(),
        1,
        "select+where_expr should find exactly one person"
    );
    assert_eq!(items[0]["handle"].as_str(), Some(handle.as_str()));
}

#[tokio::test]
async fn query_count_returns_total_count() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    let handle = expect_created_handle(harness.call("create_person", json!({})).await);

    let result = expect_ok(
        harness
            .call(
                "query_object",
                json!({
                    "object_type": "person",
                    "where_expr": format!(r#"handle == "{handle}""#),
                    "count": true,
                }),
            )
            .await,
    );

    assert_eq!(
        result["total_count"].as_u64(),
        Some(1),
        "count=true should report exactly one matching row"
    );
}

#[tokio::test]
async fn get_tree_info_reports_object_counts() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    expect_created_handle(harness.call("create_person", json!({})).await);

    let info = expect_ok(harness.call("get_tree_info", json!({})).await);
    assert!(
        info["object_counts"]["people"].as_u64().unwrap_or(0) >= 1,
        "expected at least the one created person counted, got {info}"
    );
}
