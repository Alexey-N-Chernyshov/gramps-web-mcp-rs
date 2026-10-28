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

/// Manages a single Gramps Web JWT (one fixed username/password) with automatic refresh.
#[derive(Clone)]
pub struct AuthManager {
    api_url: Url,
    username: String,
    password: SecretString,
    client: reqwest::Client,
    token: Arc<Mutex<Option<Token>>>,
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
            token: Arc::new(Mutex::new(None)),
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

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            tracing::error!(status, %body, "JWT authentication failed");
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

        tracing::debug!(expires_at = %expires_at, "JWT token obtained");
        Ok(Token {
            value: access,
            expires_at,
        })
    }
}

#[async_trait]
impl TokenSource for AuthManager {
    /// Returns a valid Bearer token, refreshing if expired.
    async fn get_token(&self) -> Result<String> {
        let mut guard = self.token.lock().await;

        if let Some(ref t) = *guard {
            if chrono::Utc::now() < t.expires_at - chrono::Duration::seconds(30) {
                tracing::trace!("using cached JWT token");
                return Ok(t.value.clone());
            }
        }

        tracing::debug!("JWT token expired or missing, refreshing");
        let token = self.fetch_token().await?;
        let value = token.value.clone();
        *guard = Some(token);
        Ok(value)
    }
}

/// Supplies a fixed Gramps Web access token given directly via
/// `GRAMPS_ACCESS_TOKEN`, for instances where local username/password login
/// is disabled (SSO-only). Unlike `AuthManager`, there's no way to refresh
/// this token — once it expires, calls fail until the token is replaced.
#[derive(Clone)]
pub struct StaticTokenSource {
    token: SecretString,
}

impl StaticTokenSource {
    pub fn new(token: SecretString) -> Self {
        Self { token }
    }
}

#[async_trait]
impl TokenSource for StaticTokenSource {
    async fn get_token(&self) -> Result<String> {
        Ok(self.token.expose_secret().to_string())
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
    async fn static_token_source_returns_configured_token() {
        let source = StaticTokenSource::new("fixed-token".to_string().into());
        assert_eq!(source.get_token().await.unwrap(), "fixed-token");
    }
}
