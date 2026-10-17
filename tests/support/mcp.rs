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
//! connected over a `tokio::io::duplex` pipe — no HTTP, no Docker. Exercises exactly what a
//! real MCP client sees: tool names, JSON-RPC argument parsing, and `server.rs`'s own early
//! validation and error mapping, none of which the `tools/*`-level integration tests (which
//! call straight into `tools::*` functions) touch.

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
pub const UNREACHABLE_API_URL: &str = "http://127.0.0.1:9";

/// A `Config` that builds a valid `GrampsMcpServer` without touching the network:
/// `GRAMPS_API_URL` points at [`UNREACHABLE_API_URL`], and construction itself never makes a
/// request. `readonly` is the only field callers typically need to override.
pub fn test_config(readonly: bool) -> Config {
    Config {
        gramps_api_url: UNREACHABLE_API_URL
            .parse()
            .expect("UNREACHABLE_API_URL should be a valid URL"),
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

    pub async fn list_tools(&self) -> Vec<Tool> {
        self.client
            .list_all_tools()
            .await
            .expect("tools/list should succeed")
    }
}

/// Asserts `result` succeeded both at the protocol level and as a tool result, and returns
/// its JSON body (every tool in this server replies with one `ContentBlock::text` holding
/// pretty-printed JSON — see `ok_json` in `src/server.rs`). Every tool that returns JSON also
/// reaches Gramps Web, so this is unused by the no-Gramps-Web tests in this PR — kept here for
/// the Docker-backed MCP-level tests (part 2) that call real tools end to end.
#[allow(dead_code)]
pub fn expect_ok(result: Result<CallToolResult, ServiceError>) -> serde_json::Value {
    let result = result.expect("call_tool should succeed at the protocol level");
    assert_ne!(
        result.is_error,
        Some(true),
        "expected a successful tool result, got an error result: {result:?}"
    );
    let text = result
        .content
        .first()
        .and_then(|c| c.as_text())
        .map(|t| t.text.as_str())
        .expect("expected a text content block");
    serde_json::from_str(text).expect("tool result text should be valid JSON")
}

/// Asserts `result` reached the tool (no protocol-level error) but the tool itself reported
/// a `CallToolResult::error`, and returns the concatenated text of its content blocks.
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
