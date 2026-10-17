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

//! In-process MCP-level test harness: runs `GrampsMcpServer` and a bare rmcp client
//! connected over a `tokio::io::duplex` pipe — no HTTP. Exercises exactly what a real MCP
//! client sees: tool names, JSON-RPC argument parsing, and `server.rs`'s own validation and
//! error mapping, none of which calling straight into `tools::*` functions touches.
//!
//! [`test_config`]/[`McpHarness::new`] point at an unreachable URL, for `mcp_server_tests.rs`'s
//! no-Docker tests of `server.rs`'s own logic. [`McpHarness::connect`] points at a real
//! backend — a `TestFixture`'s tree-scoped user — for the `tests/*_tests.rs` integration
//! suite, which exercises the same tools end to end against real Gramps Web.

use gramps_web_mcp_rs::{
    config::{Config, TransportMode},
    server::GrampsMcpServer,
};
use rmcp::{
    model::{CallToolRequestParams, CallToolResult, Tool},
    service::{RunningService, ServiceError},
    RoleClient, ServiceExt,
};

/// A base URL nothing is listening on. Any validation path under test that accidentally
/// issues a real HTTP request fails fast with a connection-refused error instead of hanging
/// or (worse) silently "succeeding" against a server that was never meant to be reached —
/// which is the whole point of these tests: every scenario here must be rejected by
/// `server.rs` itself, before any Gramps Web request is attempted.
#[allow(dead_code)]
pub const UNREACHABLE_API_URL: &str = "http://127.0.0.1:9";

/// A `Config` that builds a valid `GrampsMcpServer` without touching the network:
/// `GRAMPS_API_URL` points at [`UNREACHABLE_API_URL`], and construction itself never makes a
/// request. `readonly` is the only field callers typically need to override.
#[allow(dead_code)]
pub fn test_config(readonly: bool) -> Config {
    test_config_for(UNREACHABLE_API_URL, readonly)
}

/// Like [`test_config`], but against an arbitrary `base_url` instead of the fixed unreachable
/// one — the common defaults every `Config` in this harness starts from. `gramps_username`/
/// `gramps_password` are placeholders here; [`McpHarness::connect`] overrides them with a
/// real `TestFixture` user.
fn test_config_for(base_url: &str, readonly: bool) -> Config {
    Config {
        gramps_api_url: base_url.parse().expect("base_url should be a valid URL"),
        gramps_username: Some("svc".to_string()),
        gramps_password: Some("pw".to_string().into()),
        gramps_readonly: readonly,
        mcp_transport: TransportMode::Stdio,
        mcp_http_host: "127.0.0.1".to_string(),
        mcp_http_port: 0,
        mcp_auth_token: None,
        mcp_allowed_hosts: None,
        mcp_allowed_origins: None,
        mcp_keep_alive: 300,
        mcp_max_media_bytes: 50 * 1024 * 1024,
        mcp_oauth_enabled: false,
        mcp_oauth_storage_path: "/tmp/unused-mcp-harness-tests.json".to_string(),
        mcp_oauth_storage_key: None,
        mcp_oauth_public_url: None,
    }
}

/// An in-process MCP session: a real `GrampsMcpServer` and a bare rmcp client, wired together
/// over a `tokio::io::duplex` pipe instead of stdio or HTTP.
pub struct McpHarness {
    client: RunningService<RoleClient, ()>,
}

impl McpHarness {
    /// Connects to a real backend (a `TestFixture`'s tree-scoped user, or any other reachable
    /// Gramps Web instance) — for the `tests/*_tests.rs` integration suite, as opposed to
    /// [`test_config`]'s unreachable URL used by `mcp_server_tests.rs`'s no-Gramps-Web tests.
    #[allow(dead_code)]
    pub async fn connect(base_url: &str, username: &str, password: &str) -> Self {
        Self::new(Config {
            gramps_username: Some(username.to_string()),
            gramps_password: Some(password.to_string().into()),
            ..test_config_for(base_url, false)
        })
        .await
    }

    pub async fn new(config: Config) -> Self {
        let (server_transport, client_transport) = tokio::io::duplex(64 * 1024);

        let server = GrampsMcpServer::new(config).expect("test config should build a valid server");
        tokio::spawn(async move {
            let running = server
                .serve(server_transport)
                .await
                .expect("server should initialize over the in-process duplex transport");
            let _ = running.waiting().await;
        });

        let client = ()
            .serve(client_transport)
            .await
            .expect("client should initialize over the in-process duplex transport");

        Self { client }
    }

    /// Calls a tool by name with JSON object arguments (non-object `arguments`, e.g. passing
    /// a bare array or string, is treated as no arguments — exercise malformed shapes through
    /// a tool's own fields instead).
    pub async fn call(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<CallToolResult, ServiceError> {
        let arguments = arguments.as_object().cloned().unwrap_or_default();
        self.client
            .call_tool(CallToolRequestParams::new(name.to_string()).with_arguments(arguments))
            .await
    }

    #[allow(dead_code)]
    pub async fn list_tools(&self) -> Vec<Tool> {
        self.client
            .list_all_tools()
            .await
            .expect("tools/list should succeed")
    }
}

/// Asserts `result` succeeded both at the protocol level and as a tool result, and returns
/// the raw text of its first content block. Most tools reply with one JSON-bearing block
/// (use [`expect_ok`] for those); `create_*` and `delete_object`/`merge_*` reply with a plain
/// sentence instead ("Created person with handle: ...", "Deleted person ...", "Merged person
/// ... into ...") — this is the one to use for those.
#[allow(dead_code)]
pub fn expect_success(result: Result<CallToolResult, ServiceError>) -> String {
    let result = result.expect("call_tool should succeed at the protocol level");
    assert_ne!(
        result.is_error,
        Some(true),
        "expected a successful tool result, got an error result: {result:?}"
    );
    result
        .content
        .first()
        .and_then(|c| c.as_text())
        .map(|t| t.text.clone())
        .expect("expected a text content block")
}

/// Asserts `result` succeeded both at the protocol level and as a tool result, and returns
/// its JSON body (most tools reply with one `ContentBlock::text` holding JSON — see `ok_json`
/// in `src/server.rs`). For `create_*`/`delete_object`/`merge_*`'s plain-sentence replies, use
/// [`expect_success`] instead.
#[allow(dead_code)]
pub fn expect_ok(result: Result<CallToolResult, ServiceError>) -> serde_json::Value {
    let text = expect_success(result);
    serde_json::from_str(&text).expect("tool result text should be valid JSON")
}

/// For any `create_*` tool: asserts success and extracts the handle from its
/// "Created ... with handle: <handle>" reply.
#[allow(dead_code)]
pub fn expect_created_handle(result: Result<CallToolResult, ServiceError>) -> String {
    let text = expect_success(result);
    text.rsplit("handle: ")
        .next()
        .expect("expected '...handle: <handle>' in the response text")
        .trim()
        .to_string()
}

/// For `get_media_file`: asserts success and returns (the JSON summary block, the decoded
/// bytes of the embedded resource block).
#[allow(dead_code)]
pub fn expect_media_file(
    result: Result<CallToolResult, ServiceError>,
) -> (serde_json::Value, Vec<u8>) {
    let result = result.expect("call_tool should succeed at the protocol level");
    assert_ne!(
        result.is_error,
        Some(true),
        "expected a successful tool result, got an error result: {result:?}"
    );
    let summary_text = result
        .content
        .first()
        .and_then(|c| c.as_text())
        .map(|t| t.text.as_str())
        .expect("expected a text summary block");
    let summary: serde_json::Value =
        serde_json::from_str(summary_text).expect("summary should be valid JSON");

    let resource = result
        .content
        .get(1)
        .and_then(|c| c.as_resource())
        .expect("expected an embedded resource block");
    let bytes = match &resource.resource {
        rmcp::model::ResourceContents::BlobResourceContents { blob, .. } => {
            use base64::Engine as _;
            base64::engine::general_purpose::STANDARD
                .decode(blob)
                .expect("blob should be valid base64")
        }
        other => panic!("expected a blob resource, got: {other:?}"),
    };

    (summary, bytes)
}

/// Asserts `result` reached the tool (no protocol-level error) but the tool itself reported
/// a `CallToolResult::error`, and returns the concatenated text of its content blocks.
#[allow(dead_code)]
pub fn expect_error_text(result: Result<CallToolResult, ServiceError>) -> String {
    let result = result.expect("call_tool should succeed at the protocol level");
    assert_eq!(
        result.is_error,
        Some(true),
        "expected a CallToolResult::error, got: {result:?}"
    );
    result
        .content
        .iter()
        .filter_map(|c| c.as_text())
        .map(|t| t.text.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}
