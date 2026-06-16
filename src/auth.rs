use base64::Engine as _;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::config::Config;

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

#[derive(Debug, Clone)]
struct Token {
    value: String,
    expires_at: chrono::DateTime<chrono::Utc>,
}

/// Manages Gramps Web JWT tokens with automatic refresh.
#[derive(Clone)]
pub struct AuthManager {
    config: Config,
    client: reqwest::Client,
    token: Arc<Mutex<Option<Token>>>,
}

impl AuthManager {
    pub fn new(config: Config, client: reqwest::Client) -> Self {
        Self {
            config,
            client,
            token: Arc::new(Mutex::new(None)),
        }
    }

    /// Returns a valid Bearer token, refreshing if expired.
    pub async fn get_token(&self) -> Result<String> {
        let mut guard = self.token.lock().await;

        if let Some(ref t) = *guard {
            if chrono::Utc::now() < t.expires_at - chrono::Duration::seconds(30) {
                return Ok(t.value.clone());
            }
        }

        let token = self.fetch_token().await?;
        let value = token.value.clone();
        *guard = Some(token);
        Ok(value)
    }

    async fn fetch_token(&self) -> Result<Token> {
        let url = format!(
            "{}/api/token/",
            self.config.gramps_api_url.trim_end_matches('/')
        );

        let resp = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "username": self.config.gramps_username,
                "password": self.config.gramps_password,
            }))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
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

        Ok(Token {
            value: access,
            expires_at,
        })
    }
}

/// Decode the `exp` claim from a JWT payload.
fn decode_exp(token: &str) -> chrono::DateTime<chrono::Utc> {
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
}
