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

use async_trait::async_trait;
use base64::Engine as _;
use secrecy::{ExposeSecret, SecretString};
use std::sync::Arc;
use tokio::sync::Mutex;
use url::Url;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("authentication failed: {0}")]
    Failed(String),
    #[error("invalid token response: {0}")]
    InvalidResponse(String),
    #[error(transparent)]
    Http(#[from] reqwest::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Supplies a valid Gramps Web Bearer token for `GrampsClient` to use on every
/// request. `AuthManager` (this file) authenticates once with a fixed
/// username/password; `oauth::OAuthSession` (OAuth mode) resolves the token
/// for whichever user is attached to the current request instead.
#[async_trait]
pub trait TokenSource: Send + Sync {
    async fn get_token(&self) -> Result<String>;
}

#[derive(Debug, Clone)]
struct Token {
    value: String,
    expires_at: chrono::DateTime<chrono::Utc>,
}

/// A `Token` cache shared by `AuthManager` and `SyncTokenSource`: returns the
/// cached value if it's not within 30 seconds of expiring, otherwise calls
/// `fetch` for a fresh one and caches that instead.
#[derive(Clone, Default)]
struct TokenCache(Arc<Mutex<Option<Token>>>);

impl TokenCache {
    async fn get_or_fetch<F, Fut>(&self, fetch: F) -> Result<String>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<Token>>,
    {
        let mut guard = self.0.lock().await;

        if let Some(ref t) = *guard {
            if chrono::Utc::now() < t.expires_at - chrono::Duration::seconds(30) {
                tracing::trace!("using cached token");
                return Ok(t.value.clone());
            }
        }

        tracing::debug!("token expired or missing, refreshing");
        let token = fetch().await?;
        let value = token.value.clone();
        *guard = Some(token);
        Ok(value)
    }
}

/// Parses a `TokenSchema` response body (`{"access_token": ...}`, as returned by
/// both `/api/token/` and `/api/token/sync/`) into a cacheable `Token`.
async fn token_from_response(resp: reqwest::Response, what: &str) -> Result<Token> {
    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        tracing::error!(status, %body, "{what} failed");
        return Err(Error::Failed(format!("status={status}: {body}")));
    }

    let raw = resp
        .text()
        .await
        .map_err(|e| Error::Failed(format!("failed to read token response: {e}")))?;
    let body: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|e| Error::Failed(format!("token JSON error ({e}); body: {raw}")))?;
    let access = body["access_token"]
        .as_str()
        .ok_or_else(|| Error::InvalidResponse("no access_token in response".into()))?
        .to_string();

    let expires_at = decode_exp(&access);
    tracing::debug!(expires_at = %expires_at, "{what} succeeded");
    Ok(Token {
        value: access,
        expires_at,
    })
}

/// Manages a single Gramps Web JWT (one fixed username/password) with automatic refresh.
#[derive(Clone)]
pub struct AuthManager {
    api_url: Url,
    username: String,
    password: SecretString,
    client: reqwest::Client,
    token: TokenCache,
}

impl AuthManager {
    pub fn new(
        api_url: Url,
        username: String,
        password: SecretString,
        client: reqwest::Client,
    ) -> Self {
        Self {
            api_url,
            username,
            password,
            client,
            token: TokenCache::default(),
        }
    }

    async fn fetch_token(&self) -> Result<Token> {
        let url = format!("{}/api/token/", self.api_url.as_str().trim_end_matches('/'));

        tracing::debug!("fetching JWT token from {url}");
        let resp = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "username": self.username,
                "password": self.password.expose_secret(),
            }))
            .send()
            .await?;

        token_from_response(resp, "JWT authentication").await
    }
}

#[async_trait]
impl TokenSource for AuthManager {
    /// Returns a valid Bearer token, refreshing if expired.
    async fn get_token(&self) -> Result<String> {
        self.token.get_or_fetch(|| self.fetch_token()).await
    }
}

/// Supplies a Gramps Web access token by exchanging a persistent **sync
/// token** — created in Gramps Web under Settings → User settings → Account
/// → Access tokens → Add sync device, and given to this server via
/// `GRAMPS_SYNC_TOKEN` — for a short-lived JWT via `POST /api/token/sync/`.
/// Unlike that one-time exchange, the sync token itself doesn't expire, so
/// this refreshes automatically the same way `AuthManager` does; it just
/// re-exchanges the sync token instead of a username/password. Works even
/// when local username/password login is disabled (SSO-only instances),
/// since the exchange only checks the sync token itself.
#[derive(Clone)]
pub struct SyncTokenSource {
    api_url: Url,
    sync_token: SecretString,
    client: reqwest::Client,
    token: TokenCache,
}

impl SyncTokenSource {
    pub fn new(api_url: Url, sync_token: SecretString, client: reqwest::Client) -> Self {
        Self {
            api_url,
            sync_token,
            client,
            token: TokenCache::default(),
        }
    }

    async fn fetch_token(&self) -> Result<Token> {
        let url = format!(
            "{}/api/token/sync/",
            self.api_url.as_str().trim_end_matches('/')
        );

        tracing::debug!("exchanging sync token for a JWT at {url}");
        let resp = self
            .client
            .post(&url)
            .json(&serde_json::json!({ "token": self.sync_token.expose_secret() }))
            .send()
            .await?;

        token_from_response(resp, "sync token exchange").await
    }
}

#[async_trait]
impl TokenSource for SyncTokenSource {
    async fn get_token(&self) -> Result<String> {
        self.token.get_or_fetch(|| self.fetch_token()).await
    }
}

/// Decode the `exp` claim from a JWT payload.
pub(crate) fn decode_exp(token: &str) -> chrono::DateTime<chrono::Utc> {
    decode_exp_inner(token).unwrap_or_else(|| chrono::Utc::now() + chrono::Duration::minutes(15))
}

fn decode_exp_inner(token: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    let payload = token.split('.').nth(1)?;
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .ok()?;
    let json: serde_json::Value = serde_json::from_slice(&decoded).ok()?;
    let exp = json["exp"].as_i64()?;
    chrono::DateTime::from_timestamp(exp, 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;

    fn make_jwt(exp: i64) -> String {
        let payload = serde_json::json!({ "exp": exp, "sub": "testuser" });
        let encoded = URL_SAFE_NO_PAD.encode(payload.to_string());
        format!("eyJhbGciOiJIUzI1NiJ9.{encoded}.sig")
    }

    #[test]
    fn decode_exp_valid() {
        let exp = 9_999_999_999i64;
        let jwt = make_jwt(exp);
        let dt = decode_exp_inner(&jwt).expect("should decode");
        assert_eq!(dt.timestamp(), exp);
    }

    #[test]
    fn decode_exp_no_exp_claim() {
        let payload = URL_SAFE_NO_PAD.encode(r#"{"sub":"user"}"#);
        let jwt = format!("header.{payload}.sig");
        assert!(decode_exp_inner(&jwt).is_none());
    }

    #[test]
    fn decode_exp_not_a_jwt() {
        assert!(decode_exp_inner("notajwt").is_none());
        assert!(decode_exp_inner("a.b.c").is_none()); // b is not valid base64 JSON
    }

    #[test]
    fn decode_exp_fallback_used_on_bad_token() {
        let before = chrono::Utc::now();
        let dt = decode_exp("not.a.jwt");
        // fallback adds 15 minutes
        assert!(dt > before);
    }

    #[tokio::test]
    async fn sync_token_source_exchanges_for_a_jwt_and_caches_it() {
        use axum::{routing::post, Json, Router};
        use std::sync::atomic::{AtomicUsize, Ordering};

        static CALLS: AtomicUsize = AtomicUsize::new(0);

        async fn token_sync(Json(body): Json<serde_json::Value>) -> Json<serde_json::Value> {
            CALLS.fetch_add(1, Ordering::SeqCst);
            assert_eq!(body["token"], "sync-tok");
            Json(serde_json::json!({ "access_token": make_jwt(9_999_999_999) }))
        }

        let app = Router::new().route("/api/token/sync/", post(token_sync));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let source = SyncTokenSource::new(
            format!("http://{addr}").parse().unwrap(),
            "sync-tok".to_string().into(),
            reqwest::Client::new(),
        );

        let token = source.get_token().await.unwrap();
        assert!(token.starts_with("eyJ"));
        // Cached on the second call: no second exchange.
        source.get_token().await.unwrap();
        assert_eq!(CALLS.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn sync_token_source_surfaces_exchange_failure() {
        use axum::{http::StatusCode, routing::post, Router};

        async fn token_sync() -> StatusCode {
            StatusCode::UNAUTHORIZED
        }

        let app = Router::new().route("/api/token/sync/", post(token_sync));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let source = SyncTokenSource::new(
            format!("http://{addr}").parse().unwrap(),
            "bad-tok".to_string().into(),
            reqwest::Client::new(),
        );

        let err = source.get_token().await.unwrap_err();
        assert!(matches!(err, Error::Failed(_)));
    }
}
