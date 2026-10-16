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

//! Covers the shared-host deployment (`MCP_OAUTH_PUBLIC_URL` carrying a
//! path, e.g. `https://gramps.example.com/mcp`): OAuth helper routes must
//! nest under that path instead of the bare root, and the two discovery
//! documents must live at the RFC 8414/9728 path-aware well-known URI.
//! Doesn't need a real Gramps Web instance — DCR, `/authorize` (GET) and
//! discovery don't call out to it.

use std::sync::Arc;

use axum::{extract::Request, routing::get, Router};
use gramps_web_mcp_rs::{client::GrampsClient, oauth};

async fn start_test_server(issuer: &str) -> String {
    let store_path = std::env::temp_dir().join(format!("oauth-prefix-test-{}.json", uuid_like()));
    let key = {
        use base64::Engine as _;
        base64::engine::general_purpose::STANDARD.encode([7u8; 32])
    };
    let store = oauth::Store::open(&store_path, &key).await.unwrap();
    let state = Arc::new(oauth::OAuthState {
        store,
        api_url: "http://127.0.0.1:1".parse().unwrap(),
        http: reqwest::Client::new(),
        issuer: issuer.parse().expect("test issuer should be a valid URL"),
        max_media_bytes: 50 * 1024 * 1024,
    });

    // Mirrors the real binary's router shape (src/main.rs): the MCP
    // transport nested at a fixed `/mcp`, OAuth's own routes merged in
    // alongside it, both behind `identity_middleware` for `/mcp`.
    let app = Router::new()
        .nest_service(
            "/mcp",
            Router::new().route(
                "/",
                get(|req: Request| async move {
                    match req.extensions().get::<GrampsClient>() {
                        Some(_) => "ok",
                        None => "no client attached",
                    }
                }),
            ),
        )
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            oauth::identity_middleware,
        ))
        .merge(oauth::router(state));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{addr}")
}

fn uuid_like() -> String {
    use rand::Rng;
    let mut buf = [0u8; 8];
    rand::rng().fill_bytes(&mut buf);
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

#[tokio::test]
async fn path_prefixed_issuer_nests_oauth_routes_under_its_own_path() {
    let base_url = start_test_server("http://test-issuer.invalid/mcp").await;
    let http = reqwest::Client::new();

    // Protected resource metadata lives at the RFC 9728 path-aware
    // location, not at the bare root.
    let prm: serde_json::Value = http
        .get(format!(
            "{base_url}/.well-known/oauth-protected-resource/mcp"
        ))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(prm["resource"], "http://test-issuer.invalid/mcp");

    // Authorization server metadata likewise, and its endpoints all live
    // under the issuer's own path.
    let asm: serde_json::Value = http
        .get(format!(
            "{base_url}/.well-known/oauth-authorization-server/mcp"
        ))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(asm["issuer"], "http://test-issuer.invalid/mcp");
    assert_eq!(
        asm["authorization_endpoint"],
        "http://test-issuer.invalid/mcp/authorize"
    );
    assert_eq!(
        asm["token_endpoint"],
        "http://test-issuer.invalid/mcp/token"
    );
    assert_eq!(
        asm["registration_endpoint"],
        "http://test-issuer.invalid/mcp/register"
    );

    // DCR and /authorize are reachable under the prefix...
    let register: serde_json::Value = http
        .post(format!("{base_url}/mcp/register"))
        .json(&serde_json::json!({ "redirect_uris": ["http://localhost:9999/callback"] }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let client_id = register["client_id"].as_str().unwrap();

    let form_html = http
        .get(format!("{base_url}/mcp/authorize"))
        .query(&[
            ("client_id", client_id),
            ("redirect_uri", "http://localhost:9999/callback"),
            ("code_challenge", "dummy"),
            ("code_challenge_method", "S256"),
        ])
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(
        form_html.contains(r#"action="/mcp/authorize""#),
        "form should post back to the prefixed path, got: {form_html}"
    );

    // ...and NOT at the old bare-root paths, proving the prefix actually
    // took effect rather than silently falling back to root.
    let root_register = http
        .post(format!("{base_url}/register"))
        .json(&serde_json::json!({ "redirect_uris": [] }))
        .send()
        .await
        .unwrap();
    assert_eq!(root_register.status(), reqwest::StatusCode::NOT_FOUND);

    let root_prm = http
        .get(format!("{base_url}/.well-known/oauth-protected-resource"))
        .send()
        .await
        .unwrap();
    assert_eq!(root_prm.status(), reqwest::StatusCode::NOT_FOUND);

    // The 401 challenge on the actual MCP endpoint points at the
    // path-aware metadata URL too.
    let unauth = http.get(format!("{base_url}/mcp")).send().await.unwrap();
    assert_eq!(unauth.status(), reqwest::StatusCode::UNAUTHORIZED);
    let challenge = unauth
        .headers()
        .get("www-authenticate")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(
        challenge.contains(
            "resource_metadata=\"http://test-issuer.invalid/.well-known/oauth-protected-resource/mcp\""
        ),
        "got: {challenge}"
    );

    // And the bare-root issuer case (the pre-existing, single-host
    // deployment) still reduces to the original, unprefixed paths.
    let base_url_root = start_test_server("http://root-issuer.invalid").await;
    let root_asm: serde_json::Value = http
        .get(format!(
            "{base_url_root}/.well-known/oauth-authorization-server"
        ))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        root_asm["authorization_endpoint"],
        "http://root-issuer.invalid/authorize"
    );
}
