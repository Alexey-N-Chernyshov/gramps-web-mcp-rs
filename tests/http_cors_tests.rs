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

//! `src/main.rs` only mounts a `CorsLayer` when `MCP_ALLOWED_ORIGINS` is
//! set, and then only reflects origins from that list. With it unset (the
//! default), no CORS headers are emitted at all. These tests build the
//! same middleware chains `src/main.rs` wires up around `/mcp`
//! (`http_auth::bearer_auth_middleware` for the static-token mode,
//! `oauth::identity_middleware` for OAuth mode) with a stand-in `/mcp`
//! handler, since the real rmcp service isn't needed to exercise our own
//! CORS wiring. The companion rmcp-level Origin check
//! (`StreamableHttpServerConfig::enforce_origin_validation`/
//! `with_allowed_origins`) is covered separately in
//! `tests/http_origin_tests.rs`, against the real `/mcp` service.

use std::sync::Arc;

use axum::{http::Method, routing::any, Router};
use gramps_web_mcp_rs::{config::origin_matches, http_auth::bearer_auth_middleware, oauth};
use secrecy::SecretString;
use tower_http::cors::{AllowOrigin, CorsLayer};

fn cors_layer(allowed: Vec<String>) -> CorsLayer {
    CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::DELETE])
        .allow_headers(tower_http::cors::Any)
        .allow_origin(AllowOrigin::predicate(move |origin, _parts| {
            origin.to_str().is_ok_and(|o| origin_matches(o, &allowed))
        }))
}

async fn spawn(app: Router) -> String {
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

fn static_token_app(token: &str) -> Router {
    Router::new()
        .route("/mcp", any(|| async { "ok" }))
        .route_layer(axum::middleware::from_fn_with_state(
            SecretString::from(token.to_string()),
            bearer_auth_middleware,
        ))
}

async fn oauth_app() -> Router {
    let store_path = std::env::temp_dir().join(format!("cors-test-{}.json", std::process::id()));
    let key = {
        use base64::Engine as _;
        base64::engine::general_purpose::STANDARD.encode([7u8; 32])
    };
    let store = oauth::Store::open(&store_path, &key)
        .await
        .expect("oauth store should open at a fresh temp path");
    let state = Arc::new(oauth::OAuthState {
        store,
        api_url: "http://gramps.invalid"
            .parse()
            .expect("test api url should be a valid URL"),
        http: reqwest::Client::new(),
        issuer: "https://mcp.invalid"
            .parse()
            .expect("test issuer should be a valid URL"),
        max_media_bytes: 50 * 1024 * 1024,
    });

    Router::new()
        .route("/mcp", any(|| async { "ok" }))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            oauth::identity_middleware,
        ))
        .merge(oauth::router(state))
}

#[tokio::test]
async fn static_token_mode_sends_no_cors_headers_on_preflight() {
    let base_url = spawn(static_token_app("secret-token")).await;
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
    let base_url = spawn(static_token_app("secret-token")).await;
    let http = reqwest::Client::new();

    // No bearer token -> still 401, same as before CORS removal.
    let unauth = http.post(format!("{base_url}/mcp")).send().await.unwrap();
    assert_eq!(unauth.status(), reqwest::StatusCode::UNAUTHORIZED);
    assert!(unauth
        .headers()
        .get("access-control-allow-origin")
        .is_none());

    // Correct bearer token -> reaches the handler, same as before.
    let authed = http
        .post(format!("{base_url}/mcp"))
        .bearer_auth("secret-token")
        .send()
        .await
        .unwrap();
    assert_eq!(authed.status(), reqwest::StatusCode::OK);
    assert!(authed
        .headers()
        .get("access-control-allow-origin")
        .is_none());
}

#[tokio::test]
async fn oauth_mode_sends_no_cors_headers_on_preflight() {
    let base_url = spawn(oauth_app().await).await;
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
async fn oauth_mode_post_without_origin_is_unaffected() {
    let base_url = spawn(oauth_app().await).await;
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
}

#[tokio::test]
async fn configured_origins_are_reflected_on_preflight() {
    let app = static_token_app("secret-token")
        .layer(cors_layer(vec!["https://good.example".to_string()]));
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
    let app = static_token_app("secret-token")
        .layer(cors_layer(vec!["https://good.example".to_string()]));
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
