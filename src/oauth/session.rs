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

//! `TokenSource` for a logged-in OAuth user: produces a valid Gramps Web
//! access token for whichever person this MCP session belongs to, silently
//! refreshing it (via Gramps' own `/api/token/refresh/`, never the password)
//! when the cached one is stale.

use std::sync::Arc;

use async_trait::async_trait;
use url::Url;

use crate::auth::{self, TokenSource};

use super::store::Store;

pub struct OAuthSession {
    store: Arc<Store>,
    api_url: Url,
    http: reqwest::Client,
    /// The Bearer access token the client presented on this request — our
    /// lookup key into the store's session table.
    access_token: String,
}

impl OAuthSession {
    pub fn new(
        store: Arc<Store>,
        api_url: Url,
        http: reqwest::Client,
        access_token: String,
    ) -> Self {
        Self {
            store,
            api_url,
            http,
            access_token,
        }
    }
}

#[async_trait]
impl TokenSource for OAuthSession {
    async fn get_token(&self) -> auth::Result<String> {
        let session = self
            .store
            .session_for_access_token(&self.access_token)
            .await
            .ok_or_else(|| auth::Error::Failed("OAuth session not found or expired".into()))?;

        if let (Some(token), Some(expires_at)) =
            (&session.cached_access_token, session.cached_expires_at)
        {
            if chrono::Utc::now() < expires_at - chrono::Duration::seconds(30) {
                tracing::trace!("using cached Gramps access token for OAuth session");
                return Ok(token.clone());
            }
        }

        tracing::debug!("Gramps access token expired or missing, refreshing via refresh_token");
        let url = format!(
            "{}/api/token/refresh/",
            self.api_url.as_str().trim_end_matches('/')
        );
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&session.gramps_refresh_token)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            let reason = refresh_failure_reason(status.as_u16());
            tracing::warn!(%status, %body, "Gramps token refresh failed: {reason}");
            return Err(auth::Error::Failed(format!(
                "{reason} — sign in again via /authorize (status={status}: {body})"
            )));
        }

        let raw = resp
            .text()
            .await
            .map_err(|e| auth::Error::Failed(format!("failed to read refresh response: {e}")))?;
        let body: serde_json::Value = serde_json::from_str(&raw).map_err(|e| {
            auth::Error::Failed(format!("refresh response JSON error ({e}): {raw}"))
        })?;
        let access = body["access_token"]
            .as_str()
            .ok_or_else(|| {
                auth::Error::InvalidResponse("no access_token in refresh response".into())
            })?
            .to_string();
        let expires_at = auth::decode_exp(&access);

        self.store
            .update_cached_gramps_token(&self.access_token, access.clone(), expires_at)
            .await
            .map_err(|e| auth::Error::Failed(format!("failed to cache refreshed token: {e}")))?;

        Ok(access)
    }
}

/// Explains why Gramps Web's `/api/token/refresh/` rejected a refresh token.
fn refresh_failure_reason(status: u16) -> &'static str {
    match status {
        401 => "the Gramps refresh token has expired or been revoked",
        422 => {
            "Gramps could not verify the refresh token (its SECRET_KEY may have changed, \
                invalidating all previously issued tokens)"
        }
        _ => "Gramps rejected the refresh request",
    }
}

#[cfg(test)]
mod tests {
    use super::refresh_failure_reason;

    #[test]
    fn distinguishes_expired_from_unverifiable() {
        assert!(refresh_failure_reason(401).contains("expired or been revoked"));
        assert!(refresh_failure_reason(422).contains("SECRET_KEY"));
    }

    #[test]
    fn falls_back_to_generic_reason_for_other_statuses() {
        assert_eq!(
            refresh_failure_reason(500),
            "Gramps rejected the refresh request"
        );
    }
}
