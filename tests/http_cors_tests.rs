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

//! `build_app` only mounts a `CorsLayer` when `MCP_ALLOWED_ORIGINS` is set,
//! and then only reflects origins from that list. With it unset (the
//! default), no CORS headers are emitted at all. These tests exercise the
//! real router `build_app` produces, with a `gramps.invalid` API URL so
//! nothing actually reaches a Gramps Web backend — only requests that pass
//! auth/CORS would ever try, and none of these do. The companion rmcp-level
//! Origin check (`StreamableHttpServerConfig::enforce_origin_validation`/
//! `with_allowed_origins`) is covered separately in
//! `tests/http_origin_tests.rs`.

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
        mcp_oauth_storage_path: "/tmp/unused-http-cors-tests.json".to_string(),
        mcp_oauth_storage_key: None,
        mcp_oauth_public_url: None,
    }
}

fn oauth_test_config(store_path: &std::path::Path) -> Config {
    let key = {
        use base64::Engine as _;
        base64::engine::general_purpose::STANDARD.encode([7u8; 32])
    };
    Config {
        gramps_username: None,
        gramps_password: None,
        mcp_oauth_enabled: true,
        mcp_oauth_storage_path: store_path.to_string_lossy().into_owned(),
        mcp_oauth_storage_key: Some(key.into()),
        mcp_oauth_public_url: Some(
            "https://mcp.invalid"
                .parse()
                .expect("test issuer should be a valid URL"),
        ),
        ..test_config()
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
async fn static_token_mode_sends_no_cors_headers_on_preflight() {
    let config = Config {
        mcp_auth_token: Some("secret-token".to_string().into()),
        ..test_config()
    };
    let app = build_app(config).await.unwrap();
    let base_url = spawn(app).await;
    let http = reqwest::Client::new();

    let resp = http
        .request(reqwest::Method::OPTIONS, format!("{base_url}/mcp"))
        .header("Origin", "https://evil.example")
        .header("Access-Control-Request-Method", "POST")
        .send()
        .await
        .unwrap();
    assert!(
        resp.headers().get("access-control-allow-origin").is_none(),
        "expected no Access-Control-Allow-Origin header, got: {:?}",
        resp.headers()
    );
}

#[tokio::test]
async fn static_token_mode_post_without_origin_is_unaffected() {
    let config = Config {
        mcp_auth_token: Some("secret-token".to_string().into()),
        ..test_config()
    };
    let app = build_app(config).await.unwrap();
    let base_url = spawn(app).await;
    let http = reqwest::Client::new();

    // No bearer token -> still 401, same as before CORS removal.
    let unauth = http.post(format!("{base_url}/mcp")).send().await.unwrap();
    assert_eq!(unauth.status(), reqwest::StatusCode::UNAUTHORIZED);
    assert!(unauth
        .headers()
        .get("access-control-allow-origin")
        .is_none());

    // Correct bearer token -> auth passes and the request reaches the real
    // MCP service (which then rejects the empty body on its own terms —
    // irrelevant here, this test only cares that auth/CORS let it through).
    let authed = http
        .post(format!("{base_url}/mcp"))
        .bearer_auth("secret-token")
        .send()
        .await
        .unwrap();
    assert_ne!(authed.status(), reqwest::StatusCode::UNAUTHORIZED);
    assert!(authed
        .headers()
        .get("access-control-allow-origin")
        .is_none());
}

#[tokio::test]
async fn oauth_mode_sends_no_cors_headers_on_preflight() {
    let store_path = std::env::temp_dir().join(format!("cors-test-{}.json", std::process::id()));
    let app = build_app(oauth_test_config(&store_path)).await.unwrap();
    let base_url = spawn(app).await;
    let http = reqwest::Client::new();

    let resp = http
        .request(reqwest::Method::OPTIONS, format!("{base_url}/mcp"))
        .header("Origin", "https://evil.example")
        .header("Access-Control-Request-Method", "POST")
        .send()
        .await
        .unwrap();
    assert!(
        resp.headers().get("access-control-allow-origin").is_none(),
        "expected no Access-Control-Allow-Origin header, got: {:?}",
        resp.headers()
    );

    let _ = std::fs::remove_file(&store_path);
}

#[tokio::test]
async fn oauth_mode_post_without_origin_is_unaffected() {
    let store_path =
        std::env::temp_dir().join(format!("cors-test-oauth-{}.json", std::process::id()));
    let app = build_app(oauth_test_config(&store_path)).await.unwrap();
    let base_url = spawn(app).await;
    let http = reqwest::Client::new();

    // No Authorization header -> 401 with a WWW-Authenticate challenge,
    // same as before CORS removal.
    let unauth = http.post(format!("{base_url}/mcp")).send().await.unwrap();
    assert_eq!(unauth.status(), reqwest::StatusCode::UNAUTHORIZED);
    assert!(unauth.headers().contains_key("www-authenticate"));
    assert!(unauth
        .headers()
        .get("access-control-allow-origin")
        .is_none());

    let _ = std::fs::remove_file(&store_path);
}

#[tokio::test]
async fn configured_origins_are_reflected_on_preflight() {
    let config = Config {
        mcp_auth_token: Some("secret-token".to_string().into()),
        mcp_allowed_origins: Some("https://good.example".to_string()),
        ..test_config()
    };
    let app = build_app(config).await.unwrap();
    let base_url = spawn(app).await;
    let http = reqwest::Client::new();

    let resp = http
        .request(reqwest::Method::OPTIONS, format!("{base_url}/mcp"))
        .header("Origin", "https://good.example")
        .header("Access-Control-Request-Method", "POST")
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.headers().get("access-control-allow-origin"),
        Some(&reqwest::header::HeaderValue::from_static(
            "https://good.example"
        )),
        "expected the configured origin to be reflected, got: {:?}",
        resp.headers()
    );
}

#[tokio::test]
async fn unlisted_origin_gets_no_cors_headers_even_when_others_are_configured() {
    let config = Config {
        mcp_auth_token: Some("secret-token".to_string().into()),
        mcp_allowed_origins: Some("https://good.example".to_string()),
        ..test_config()
    };
    let app = build_app(config).await.unwrap();
    let base_url = spawn(app).await;
    let http = reqwest::Client::new();

    let resp = http
        .request(reqwest::Method::OPTIONS, format!("{base_url}/mcp"))
        .header("Origin", "https://evil.example")
        .header("Access-Control-Request-Method", "POST")
        .send()
        .await
        .unwrap();
    assert!(
        resp.headers().get("access-control-allow-origin").is_none(),
        "expected no Access-Control-Allow-Origin header for an unlisted origin, got: {:?}",
        resp.headers()
    );
}
