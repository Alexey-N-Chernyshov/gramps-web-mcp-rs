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

//! MCP-level tests, part 1: the tool catalog contract and every `server.rs` validation branch
//! that rejects a call before it would reach Gramps Web — exercised through a real rmcp
//! client (`tests/support/mcp.rs`), not by calling into `tools::*` directly. This is the layer
//! the `tools/*`-level integration tests (`tests/*_tests.rs`, against a real Gramps Web via
//! Docker) don't touch at all: tool names as published over MCP, JSON-RPC argument parsing,
//! and `server.rs`'s own early `CallToolResult::error` branches and network-error mapping.
//!
//! Two of `server.rs`'s validation branches are deliberately *not* duplicated here:
//! `get_object`'s handle/gramps_id branches (neither/both/tag+gramps_id) just forward
//! `get::resolve_object_lookup`'s error, and `list_objects`'s `handles` conflicts
//! (handles+rules/sort, handles>100) just forward `get::validate_handles`'s — both are
//! already thoroughly unit-tested as pure functions in `src/tools/get.rs`, and `update_*`'s
//! data validation (empty/non-object/mismatched handle) is likewise already unit-tested as
//! `validate_patch` in `src/tools/update.rs`. Testing them again here through JSON-RPC would
//! just re-verify the same pure-function logic through an extra layer of indirection.

#[path = "support/mcp.rs"]
mod mcp;

use mcp::{expect_error_text, test_config, McpHarness};
use serde_json::json;

// ── Tool catalog contract ──────────────────────────────────────────────────

#[tokio::test]
async fn list_tools_returns_all_tools_with_valid_descriptions_and_schemas() {
    let harness = McpHarness::new(test_config(false)).await;
    let tools = harness.list_tools().await;

    assert_eq!(tools.len(), 44, "expected all 44 tools, got: {tools:?}");

    for tool in &tools {
        assert!(
            tool.description.as_deref().is_some_and(|d| !d.is_empty()),
            "tool {:?} has no description",
            tool.name
        );
        assert_eq!(
            tool.input_schema.get("type"),
            Some(&json!("object")),
            "tool {:?} should have an object input schema, got: {:?}",
            tool.name,
            tool.input_schema
        );
    }

    let get_object = tools.iter().find(|t| t.name == "get_object").unwrap();
    assert_eq!(
        get_object.input_schema.get("required"),
        Some(&json!(["object_type"])),
        "get_object should only require object_type, got: {:?}",
        get_object.input_schema.get("required")
    );

    let list_objects = tools.iter().find(|t| t.name == "list_objects").unwrap();
    assert_eq!(
        list_objects.input_schema.get("required"),
        Some(&json!(["object_type"])),
        "list_objects should only require object_type, got: {:?}",
        list_objects.input_schema.get("required")
    );
}

#[tokio::test]
async fn readonly_mode_hides_write_tools() {
    let harness = McpHarness::new(test_config(true)).await;
    let tools = harness.list_tools().await;
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();

    assert!(
        !names.contains(&"create_person"),
        "readonly mode should hide create_person, got: {names:?}"
    );
    assert!(
        !names.contains(&"delete_object"),
        "readonly mode should hide delete_object, got: {names:?}"
    );
    assert!(
        !names.contains(&"merge_person"),
        "readonly mode should hide merge_person, got: {names:?}"
    );
    assert!(
        names.contains(&"get_object"),
        "readonly mode should still publish read tools, got: {names:?}"
    );
    assert!(
        names.contains(&"search"),
        "readonly mode should still publish read tools, got: {names:?}"
    );

    // Hidden, not just unlisted: calling it directly is rejected too.
    let error = expect_error_text(harness.call("create_person", json!({})).await);
    assert!(
        error.contains("tool not found"),
        "calling a readonly-hidden tool should be rejected, got: {error}"
    );
}

// ── Early validation branches (rejected before any Gramps Web request) ────

#[tokio::test]
async fn get_object_requires_object_type() {
    let harness = McpHarness::new(test_config(false)).await;
    let error = expect_error_text(harness.call("get_object", json!({"handle": "H1"})).await);
    assert!(error.contains("object_type"), "got: {error}");
}

#[tokio::test]
async fn list_objects_requires_object_type() {
    let harness = McpHarness::new(test_config(false)).await;
    let error = expect_error_text(harness.call("list_objects", json!({})).await);
    assert!(error.contains("object_type"), "got: {error}");
}

#[tokio::test]
async fn query_object_requires_object_type() {
    let harness = McpHarness::new(test_config(false)).await;
    let error = expect_error_text(harness.call("query_object", json!({})).await);
    assert!(error.contains("object_type"), "got: {error}");
}

#[tokio::test]
async fn get_filter_rules_requires_object_type() {
    let harness = McpHarness::new(test_config(false)).await;
    let error = expect_error_text(harness.call("get_filter_rules", json!({})).await);
    assert!(error.contains("object_type"), "got: {error}");
}

#[tokio::test]
async fn delete_object_requires_object_type() {
    let harness = McpHarness::new(test_config(false)).await;
    let error = expect_error_text(harness.call("delete_object", json!({"handle": "H1"})).await);
    assert!(error.contains("object_type"), "got: {error}");
}

#[tokio::test]
async fn create_media_requires_exactly_one_of_path_url_data_base64() {
    let harness = McpHarness::new(test_config(false)).await;
    let error = expect_error_text(harness.call("create_media", json!({})).await);
    assert!(
        error.contains("path") && error.contains("url") && error.contains("data_base64"),
        "got: {error}"
    );
}

#[tokio::test]
async fn create_media_rejects_invalid_base64() {
    let harness = McpHarness::new(test_config(false)).await;
    let error = expect_error_text(
        harness
            .call("create_media", json!({"data_base64": "not-valid-base64!!"}))
            .await,
    );
    assert!(error.contains("base64"), "got: {error}");
}

#[tokio::test]
async fn replace_media_file_rejects_invalid_base64() {
    let harness = McpHarness::new(test_config(false)).await;
    let error = expect_error_text(
        harness
            .call(
                "replace_media_file",
                json!({"handle": "H1", "data_base64": "not-valid-base64!!"}),
            )
            .await,
    );
    assert!(error.contains("base64"), "got: {error}");
}

#[tokio::test]
async fn unknown_tool_name_returns_a_clear_error() {
    // `ToolRouter::call` reports an unknown (or readonly-disabled) tool name as
    // `ErrorData::invalid_params("tool not found", ..)`, and `GrampsMcpServer::call_tool`
    // (src/server.rs) maps every `INVALID_PARAMS` error from the router into
    // `CallToolResult::error` rather than letting it surface as a JSON-RPC protocol error —
    // so the caller gets a readable tool result instead of an opaque "internal error".
    let harness = McpHarness::new(test_config(false)).await;
    let error = expect_error_text(harness.call("not_a_real_tool", json!({})).await);
    assert!(error.contains("tool not found"), "got: {error}");
}

// ── Network error mapping ──────────────────────────────────────────────────

#[tokio::test]
async fn network_error_maps_to_a_clear_tool_error_not_a_panic_or_protocol_error() {
    let harness = McpHarness::new(test_config(false)).await;
    let error = expect_error_text(
        harness
            .call(
                "get_object",
                json!({"object_type": "person", "handle": "H1"}),
            )
            .await,
    );
    assert!(
        error.contains("127.0.0.1:9"),
        "expected the underlying connection error to surface, got: {error}"
    );
}

// `get_goql_reference`/`get_rules_reference` are the only tools that never reach Gramps
// Web at all, so they're the one place in this file a genuinely successful call can be
// exercised without Docker. They return plain markdown rather than JSON, so this checks
// `is_error` directly instead of going through `expect_ok` (which expects JSON content).
#[tokio::test]
async fn get_goql_reference_succeeds_without_touching_gramps_web() {
    let harness = McpHarness::new(test_config(false)).await;
    let result = harness
        .call("get_goql_reference", json!({}))
        .await
        .expect("call_tool should succeed at the protocol level");
    assert_ne!(result.is_error, Some(true), "got: {result:?}");
}
