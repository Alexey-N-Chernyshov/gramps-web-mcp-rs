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
//! `get_object`'s handle/gramps_id branches forward `get::resolve_object_lookup`'s error,
//! `list_objects`'s `handles` conflicts forward `get::validate_handles`'s, and `update_*`'s
//! data checks forward `update::validate_patch`'s — all three are already thoroughly
//! unit-tested as pure functions (`src/tools/get.rs`, `src/tools/update.rs`), so the tests
//! below for those branches don't re-verify that *logic*. They check the *wiring*: that
//! `server.rs` actually calls the right function with the parsed arguments and turns its
//! `Err` into a `CallToolResult::error` the caller can read, which the pure-function unit
//! tests can't see at all (they call the function directly, never through JSON-RPC
//! argument parsing or `server.rs`'s own match arms).

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

// ── server.rs wiring for already-unit-tested pure-function validation ─────
//
// `get::resolve_object_lookup`, `get::validate_handles`, and `update::validate_patch` each
// have their own unit tests covering the validation logic itself. These tests don't repeat
// that — they check that `server.rs` actually wires the parsed arguments into the right
// function and turns its `Err` into a readable `CallToolResult::error`, without ever
// reaching Gramps Web.

#[tokio::test]
async fn get_object_rejects_neither_handle_nor_gramps_id() {
    let harness = McpHarness::new(test_config(false)).await;
    let error = expect_error_text(
        harness
            .call("get_object", json!({"object_type": "person"}))
            .await,
    );
    assert!(
        error.contains("handle") && error.contains("gramps_id"),
        "got: {error}"
    );
}

#[tokio::test]
async fn get_object_rejects_both_handle_and_gramps_id() {
    let harness = McpHarness::new(test_config(false)).await;
    let error = expect_error_text(
        harness
            .call(
                "get_object",
                json!({"object_type": "person", "handle": "H1", "gramps_id": "I0001"}),
            )
            .await,
    );
    assert!(error.contains("only one"), "got: {error}");
}

#[tokio::test]
async fn get_object_rejects_gramps_id_for_tags() {
    let harness = McpHarness::new(test_config(false)).await;
    let error = expect_error_text(
        harness
            .call(
                "get_object",
                json!({"object_type": "tag", "gramps_id": "I0001"}),
            )
            .await,
    );
    assert!(
        error.contains("gramps_id") && error.contains("handle"),
        "got: {error}"
    );
}

#[tokio::test]
async fn list_objects_rejects_handles_combined_with_rules() {
    let harness = McpHarness::new(test_config(false)).await;
    let error = expect_error_text(
        harness
            .call(
                "list_objects",
                json!({
                    "object_type": "person",
                    "handles": ["H1"],
                    "rules": {"rules": []},
                }),
            )
            .await,
    );
    assert!(
        error.contains("handles") && error.contains("rules"),
        "got: {error}"
    );
}

#[tokio::test]
async fn list_objects_rejects_handles_combined_with_sort() {
    let harness = McpHarness::new(test_config(false)).await;
    let error = expect_error_text(
        harness
            .call(
                "list_objects",
                json!({
                    "object_type": "person",
                    "handles": ["H1"],
                    "sort": ["gramps_id"],
                }),
            )
            .await,
    );
    assert!(
        error.contains("handles") && error.contains("sort"),
        "got: {error}"
    );
}

#[tokio::test]
async fn list_objects_rejects_more_than_100_handles() {
    let harness = McpHarness::new(test_config(false)).await;
    let handles: Vec<String> = (0..101).map(|i| format!("H{i}")).collect();
    let error = expect_error_text(
        harness
            .call(
                "list_objects",
                json!({"object_type": "person", "handles": handles}),
            )
            .await,
    );
    assert!(error.contains("100"), "got: {error}");
}

#[tokio::test]
async fn update_person_rejects_empty_data() {
    let harness = McpHarness::new(test_config(false)).await;
    let error = expect_error_text(
        harness
            .call("update_person", json!({"handle": "H1", "data": {}}))
            .await,
    );
    assert!(error.contains("empty"), "got: {error}");
}

#[tokio::test]
async fn update_person_rejects_non_object_data() {
    let harness = McpHarness::new(test_config(false)).await;
    let error = expect_error_text(
        harness
            .call(
                "update_person",
                json!({"handle": "H1", "data": "not-an-object"}),
            )
            .await,
    );
    assert!(error.contains("JSON object"), "got: {error}");
}

#[tokio::test]
async fn update_person_rejects_mismatched_handle_in_data() {
    let harness = McpHarness::new(test_config(false)).await;
    let error = expect_error_text(
        harness
            .call(
                "update_person",
                json!({"handle": "H1", "data": {"handle": "H2"}}),
            )
            .await,
    );
    assert!(error.contains("H1") && error.contains("H2"), "got: {error}");
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

#[tokio::test]
async fn get_rules_reference_succeeds_without_touching_gramps_web() {
    let harness = McpHarness::new(test_config(false)).await;
    let result = harness
        .call("get_rules_reference", json!({}))
        .await
        .expect("call_tool should succeed at the protocol level");
    assert_ne!(result.is_error, Some(true), "got: {result:?}");
}

// ── Media pre-flight validation (no Gramps Web) ────────────────────────────
//
// `create_media`/`replace_media_file` check size, declared-vs-sniffed MIME, and (for
// replace) expected checksums entirely locally, before ever making a request — see
// `check_size`/`resolve_mime` in `src/tools/media.rs`. These moved here from
// `media_binary_tests.rs` precisely because they don't need Docker; what's left there needs
// a real Gramps Web round trip (actually storing/reading bytes, privacy, not-found).

fn fake_jpeg(payload: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0xFFu8, 0xD8, 0xFF, 0xE0];
    bytes.extend_from_slice(payload);
    bytes
}

fn fake_png(payload: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0x89u8, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    bytes.extend_from_slice(payload);
    bytes
}

fn base64_of(bytes: &[u8]) -> String {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

#[tokio::test]
async fn create_media_rejects_mime_mismatch() {
    let harness = McpHarness::new(test_config(false)).await;
    let error = expect_error_text(
        harness
            .call(
                "create_media",
                json!({
                    "data_base64": base64_of(&fake_png(b"fake png payload")),
                    "mime": "image/jpeg",
                }),
            )
            .await,
    );
    assert!(error.contains("does not match"), "got: {error}");
}

#[tokio::test]
async fn create_media_rejects_oversized_upload() {
    let config = gramps_web_mcp_rs::config::Config {
        mcp_max_media_bytes: 10,
        ..test_config(false)
    };
    let harness = McpHarness::new(config).await;
    let error = expect_error_text(
        harness
            .call(
                "create_media",
                json!({"data_base64": base64_of(&[0xFFu8; 11])}),
            )
            .await,
    );
    assert!(error.contains("exceeds"), "got: {error}");
}

#[tokio::test]
async fn replace_media_file_rejects_mime_mismatch() {
    let harness = McpHarness::new(test_config(false)).await;
    let error = expect_error_text(
        harness
            .call(
                "replace_media_file",
                json!({
                    "handle": "some-handle",
                    "data_base64": base64_of(&fake_png(b"actually a png")),
                    "mime": "application/pdf",
                }),
            )
            .await,
    );
    assert!(error.contains("does not match"), "got: {error}");
}

#[tokio::test]
async fn replace_media_file_rejects_expected_checksum_mismatch() {
    let harness = McpHarness::new(test_config(false)).await;
    let error = expect_error_text(
        harness
            .call(
                "replace_media_file",
                json!({
                    "handle": "some-handle",
                    "data_base64": base64_of(&fake_jpeg(b"a photo")),
                    "expected_md5": "0".repeat(32),
                }),
            )
            .await,
    );
    assert!(error.contains("does not match"), "got: {error}");
}

#[tokio::test]
async fn replace_media_file_rejects_oversized_upload() {
    let config = gramps_web_mcp_rs::config::Config {
        mcp_max_media_bytes: 10,
        ..test_config(false)
    };
    let harness = McpHarness::new(config).await;
    let error = expect_error_text(
        harness
            .call(
                "replace_media_file",
                json!({"handle": "some-handle", "data_base64": base64_of(&[0xFFu8; 11])}),
            )
            .await,
    );
    assert!(error.contains("exceeds"), "got: {error}");
}

/// Unlike the checks above, this one isn't local — it needs *some* server to talk to — but
/// still no Gramps Web/Docker: a tiny fake HTTP server stands in, specifically to reproduce a
/// partial failure (binary stored, metadata PATCH failed) and confirm `create_media` surfaces
/// the handle so the record is recoverable instead of silently orphaned.
#[tokio::test]
async fn create_media_surfaces_the_handle_when_metadata_application_fails() {
    use axum::{
        extract::Path,
        http::StatusCode,
        routing::{get, post},
        Json, Router,
    };

    async fn token() -> Json<serde_json::Value> {
        Json(json!({"access_token": "faketoken"}))
    }
    async fn create_media_handler() -> (StatusCode, Json<serde_json::Value>) {
        (
            StatusCode::CREATED,
            Json(json!({"handle": "fakehandle0001"})),
        )
    }
    async fn get_media_handler(Path(_handle): Path<String>) -> Json<serde_json::Value> {
        Json(json!({
            "handle": "fakehandle0001",
            "checksum": "deadbeef",
            "path": "/fake/path.bin",
            "mime": "application/octet-stream",
            "desc": "",
            "private": false,
        }))
    }
    async fn put_media_handler() -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    let app = Router::new()
        .route("/api/token/", post(token))
        .route("/api/media/", post(create_media_handler))
        .route(
            "/api/media/{handle}",
            get(get_media_handler).put(put_media_handler),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let config = gramps_web_mcp_rs::config::Config {
        gramps_api_url: format!("http://{addr}")
            .parse()
            .expect("fake server address should be a valid URL"),
        ..test_config(false)
    };
    let harness = McpHarness::new(config).await;
    let error = expect_error_text(
        harness
            .call(
                "create_media",
                json!({"data_base64": base64_of(&[0xAAu8; 16])}),
            )
            .await,
    );
    assert!(
        error.contains("fakehandle0001"),
        "the handle must be surfaced even though metadata application failed, got: {error}"
    );
}

/// `GRAMPS_ACCESS_TOKEN` signs in with a fixed bearer token instead of
/// username/password — no `/api/token/` login call should ever happen, and
/// the configured token must be sent as-is on every request.
#[tokio::test]
async fn gramps_access_token_is_sent_without_a_login_call() {
    use axum::{extract::Request, http::StatusCode, routing::get, Json, Router};

    async fn metadata_handler(req: Request) -> (StatusCode, Json<serde_json::Value>) {
        let auth = req
            .headers()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        if auth == "Bearer fixed-access-token" {
            (StatusCode::OK, Json(json!({"person_count": 0})))
        } else {
            (StatusCode::UNAUTHORIZED, Json(json!({})))
        }
    }

    // No `/api/token/` route registered: if the client ever tried to log in
    // instead of using the static token directly, this would 404.
    let app = Router::new().route("/api/metadata/", get(metadata_handler));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let config = gramps_web_mcp_rs::config::Config {
        gramps_api_url: format!("http://{addr}")
            .parse()
            .expect("fake server address should be a valid URL"),
        gramps_username: None,
        gramps_password: None,
        gramps_access_token: Some("fixed-access-token".to_string().into()),
        ..test_config(false)
    };
    let harness = McpHarness::new(config).await;
    let result = harness
        .call("get_tree_info", json!({}))
        .await
        .expect("call should succeed at the transport level");
    assert!(
        !result.is_error.unwrap_or(false),
        "expected success, got: {result:?}"
    );
}
