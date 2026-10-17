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

//! End-to-end exercise of the OAuth 2.1 flow (DCR + PKCE + login +
//! refresh rotation) against a real Gramps Web instance. This doesn't boot
//! the full rmcp/MCP server (that's bin-only) — it mounts the same
//! `oauth::router`/`identity_middleware` the binary uses, plus one small
//! test-only route that stands in for an MCP tool call: it just proves a
//! working, correctly-identified `GrampsClient` came out the other end of
//! the middleware.

#[path = "support/helpers.rs"]
mod common;

use std::sync::Arc;

use axum::{extract::Request, routing::get, Router};
use gramps_web_mcp_rs::{client::GrampsClient, oauth, tools::get as get_tool};
use sha2::{Digest, Sha256};

async fn start_test_server(fixture: &common::TestFixture) -> (String, Arc<oauth::Store>) {
    let store_path = std::env::temp_dir().join(format!("oauth-test-{}.json", uuid_like()));
    let key = {
        use base64::Engine as _;
        base64::engine::general_purpose::STANDARD.encode([42u8; 32])
    };
    let store = oauth::Store::open(&store_path, &key)
        .await
        .expect("oauth store should open at a fresh temp path");
    let state = Arc::new(oauth::OAuthState {
        store: store.clone(),
        api_url: fixture.base_url.parse().expect("test URL should be valid"),
        http: reqwest::Client::new(),
        issuer: "http://test-issuer.invalid"
            .parse()
            .expect("static URL should be valid"),
        max_media_bytes: 50 * 1024 * 1024,
    });

    let app = Router::new()
        .route(
            "/whoami",
            get(|req: Request| async move {
                match req.extensions().get::<GrampsClient>() {
                    Some(client) => match get_tool::get_tree_info(client).await {
                        Ok(_) => "ok".to_string(),
                        Err(e) => format!("gramps call failed: {e}"),
                    },
                    None => "no client attached".to_string(),
                }
            }),
        )
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            oauth::identity_middleware,
        ))
        .merge(oauth::router(state));

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

    (format!("http://{addr}"), store)
}

fn uuid_like() -> String {
    use rand::Rng;
    let mut buf = [0u8; 8];
    rand::rng().fill_bytes(&mut buf);
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

fn pkce_pair() -> (String, String) {
    use base64::Engine as _;
    let verifier = uuid_like() + &uuid_like() + &uuid_like() + &uuid_like();
    let digest = Sha256::digest(verifier.as_bytes());
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest);
    (verifier, challenge)
}

#[tokio::test]
async fn full_authorization_code_flow_reaches_a_real_gramps_identity() {
    let fixture = common::TestFixture::new().await;
    let (base_url, _store) = start_test_server(&fixture).await;
    let http = reqwest::Client::new();

    // Dynamic Client Registration.
    let redirect_uri = "http://localhost:9999/callback";
    let register: serde_json::Value = http
        .post(format!("{base_url}/register"))
        .json(&serde_json::json!({ "redirect_uris": [redirect_uri] }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let client_id = register["client_id"].as_str().unwrap();

    let (verifier, challenge) = pkce_pair();

    // GET /authorize must render a login form containing our hidden fields.
    let form_html = http
        .get(format!("{base_url}/authorize"))
        .query(&[
            ("client_id", client_id),
            ("redirect_uri", redirect_uri),
            ("state", "teststate"),
            ("code_challenge", &challenge),
            ("code_challenge_method", "S256"),
            ("response_type", "code"),
        ])
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(form_html.contains("name=\"username\""));
    assert!(form_html.contains(client_id));

    // POST /authorize with the real Gramps test user's credentials.
    let no_redirect = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let resp = no_redirect
        .post(format!("{base_url}/authorize"))
        .form(&[
            ("client_id", client_id),
            ("redirect_uri", redirect_uri),
            ("state", "teststate"),
            ("code_challenge", &challenge),
            ("code_challenge_method", "S256"),
            ("response_type", "code"),
            ("username", fixture.username.as_str()),
            ("password", common::TEST_PASS),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), reqwest::StatusCode::SEE_OTHER);
    let location = resp
        .headers()
        .get("location")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert!(location.starts_with(redirect_uri));
    let code = location
        .split("code=")
        .nth(1)
        .unwrap()
        .split('&')
        .next()
        .unwrap();

    // Exchange the code for our access/refresh token pair.
    let token_resp: serde_json::Value = http
        .post(format!("{base_url}/token"))
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("code_verifier", &verifier),
        ])
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let access_token = token_resp["access_token"].as_str().unwrap();
    let refresh_token = token_resp["refresh_token"].as_str().unwrap().to_string();

    // The access token must resolve to a working, correctly-identified client.
    let whoami = http
        .get(format!("{base_url}/whoami"))
        .bearer_auth(access_token)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert_eq!(
        whoami, "ok",
        "expected a working Gramps identity, got: {whoami}"
    );

    // No Authorization header -> 401 with a WWW-Authenticate challenge.
    let unauth = http.get(format!("{base_url}/whoami")).send().await.unwrap();
    assert_eq!(unauth.status(), reqwest::StatusCode::UNAUTHORIZED);
    assert!(unauth.headers().contains_key("www-authenticate"));

    // Refresh rotation: old refresh token stops working, new one keeps working.
    let rotated: serde_json::Value = http
        .post(format!("{base_url}/token"))
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", &refresh_token),
        ])
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let new_access = rotated["access_token"].as_str().unwrap();
    assert_ne!(rotated["refresh_token"].as_str().unwrap(), refresh_token);

    let reused = http
        .post(format!("{base_url}/token"))
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", &refresh_token),
        ])
        .send()
        .await
        .unwrap();
    assert_eq!(reused.status(), reqwest::StatusCode::BAD_REQUEST);

    let whoami_after_rotation = http
        .get(format!("{base_url}/whoami"))
        .bearer_auth(new_access)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert_eq!(whoami_after_rotation, "ok");
}
