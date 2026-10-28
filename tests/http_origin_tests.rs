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

//! `build_app` wires `MCP_ALLOWED_ORIGINS` into rmcp's own `Origin` check on
//! `/mcp` (`StreamableHttpServerConfig::enforce_origin_validation` when
//! unset, `with_allowed_origins` when set) — not the `CorsLayer`, which only
//! governs whether a browser is *allowed to read* a response. rmcp's check
//! runs for every request, browser or not, before any MCP/session logic,
//! and rejects a disallowed `Origin` outright with `403`. These tests build
//! the real router via `build_app` (no Gramps backend needed — constructing
//! it does no network I/O) to exercise that check directly, independent of
//! the CORS layer covered in `tests/http_cors_tests.rs`.

use gramps_web_mcp_rs::{
    app::build_app,
    config::{Config, TransportMode},
};

fn test_config() -> Config {
    Config {
        gramps_api_url: "http://gramps.invalid"
            .parse()
            .expect("test api url should be a valid URL"),
        gramps_username: Some("svc".to_string()),
        gramps_password: Some("pw".to_string().into()),
        gramps_access_token: None,
        gramps_readonly: false,
        mcp_transport: TransportMode::Http,
        mcp_http_host: "127.0.0.1".to_string(),
        mcp_http_port: 0,
        mcp_auth_token: None,
        mcp_allowed_hosts: None,
        mcp_allowed_origins: None,
        mcp_keep_alive: 300,
        mcp_max_media_bytes: 50 * 1024 * 1024,
        mcp_oauth_enabled: false,
        mcp_oauth_storage_path: "/tmp/unused-http-origin-tests.json".to_string(),
        mcp_oauth_storage_key: None,
        mcp_oauth_public_url: None,
    }
}

async fn spawn(app: axum::Router) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("binding to an ephemeral local port should succeed");
    let addr = listener
        .local_addr()
        .expect("bound listener should have a local address");
    tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("test server should serve without error");
    });
    format!("http://{addr}")
}

#[tokio::test]
async fn default_enforcement_rejects_any_present_origin() {
    let app = build_app(test_config()).await.unwrap();
    let base_url = spawn(app).await;
    let http = reqwest::Client::new();

    let resp = http
        .post(format!("{base_url}/mcp"))
        .header("Origin", "https://evil.example")
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), reqwest::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn default_enforcement_allows_requests_with_no_origin_header() {
    let app = build_app(test_config()).await.unwrap();
    let base_url = spawn(app).await;
    let http = reqwest::Client::new();

    let resp = http
        .post(format!("{base_url}/mcp"))
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap();
    assert_ne!(resp.status(), reqwest::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn allowed_origins_list_lets_listed_origin_through() {
    let config = Config {
        mcp_allowed_origins: Some("https://good.example".to_string()),
        ..test_config()
    };
    let app = build_app(config).await.unwrap();
    let base_url = spawn(app).await;
    let http = reqwest::Client::new();

    let resp = http
        .post(format!("{base_url}/mcp"))
        .header("Origin", "https://good.example")
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap();
    assert_ne!(resp.status(), reqwest::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn allowed_origins_list_still_rejects_unlisted_origin() {
    let config = Config {
        mcp_allowed_origins: Some("https://good.example".to_string()),
        ..test_config()
    };
    let app = build_app(config).await.unwrap();
    let base_url = spawn(app).await;
    let http = reqwest::Client::new();

    let resp = http
        .post(format!("{base_url}/mcp"))
        .header("Origin", "https://evil.example")
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), reqwest::StatusCode::FORBIDDEN);
}
