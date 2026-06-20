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

use serde::de::DeserializeOwned;

use crate::{auth::AuthManager, config::Config};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Auth(#[from] crate::auth::Error),
    #[error("API error {status}: {body}")]
    Api { status: u16, body: String },
    #[error("not found: {0}")]
    NotFound(String),
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    #[error("parse error: {0}")]
    Parse(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Low-level HTTP client for the Gramps Web REST API.
#[derive(Clone)]
pub struct GrampsClient {
    base_url: String,
    http: reqwest::Client,
    auth: AuthManager,
}

impl GrampsClient {
    pub fn new(config: Config, http: reqwest::Client) -> Self {
        let auth = AuthManager::new(config.clone(), http.clone());
        Self {
            base_url: config.gramps_api_url.trim_end_matches('/').to_string(),
            http,
            auth,
        }
    }

    pub fn http_client(&self) -> &reqwest::Client {
        &self.http
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    async fn bearer(&self) -> Result<String> {
        Ok(self.auth.get_token().await?)
    }

    /// GET /api/{path} and deserialise the response body.
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        tracing::debug!("GET {path}");
        let token = self.bearer().await?;
        let resp = self
            .http
            .get(self.url(path))
            .header("Authorization", format!("Bearer {token}"))
            .send()
            .await?;
        self.parse(resp).await
    }

    /// POST /api/{path} with a JSON body, return deserialised response.
    pub async fn post<B: serde::Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        tracing::debug!("POST {path}");
        let token = self.bearer().await?;
        let resp = self
            .http
            .post(self.url(path))
            .header("Authorization", format!("Bearer {token}"))
            .json(body)
            .send()
            .await?;
        self.parse(resp).await
    }

    /// PUT /api/{path} with a JSON body.
    pub async fn put<B: serde::Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        tracing::debug!("PUT {path}");
        let token = self.bearer().await?;
        let resp = self
            .http
            .put(self.url(path))
            .header("Authorization", format!("Bearer {token}"))
            .json(body)
            .send()
            .await?;
        self.parse(resp).await
    }

    /// POST /api/{path} with raw bytes and explicit Content-Type, return deserialised response.
    pub async fn post_bytes<T: DeserializeOwned>(
        &self,
        path: &str,
        bytes: Vec<u8>,
        content_type: &str,
    ) -> Result<T> {
        tracing::debug!("POST {path} (bytes, content-type={content_type})");
        let token = self.bearer().await?;
        let resp = self
            .http
            .post(self.url(path))
            .header("Authorization", format!("Bearer {token}"))
            .header("Content-Type", content_type)
            .body(bytes)
            .send()
            .await?;
        self.parse(resp).await
    }

    /// DELETE /api/{path}, expects no response body.
    pub async fn delete(&self, path: &str) -> Result<()> {
        tracing::debug!("DELETE {path}");
        let token = self.bearer().await?;
        let resp = self
            .http
            .delete(self.url(path))
            .header("Authorization", format!("Bearer {token}"))
            .send()
            .await?;
        let status = resp.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            tracing::debug!(path, "not found");
            return Err(Error::NotFound(resp.url().path().to_string()));
        }
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            tracing::warn!(status = status.as_u16(), %body, "API error response");
            return Err(Error::Api {
                status: status.as_u16(),
                body,
            });
        }
        Ok(())
    }

    async fn parse<T: DeserializeOwned>(&self, resp: reqwest::Response) -> Result<T> {
        let status = resp.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            tracing::debug!(path = resp.url().path(), "not found");
            return Err(Error::NotFound(resp.url().path().to_string()));
        }
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            tracing::warn!(status = status.as_u16(), %body, "API error response");
            return Err(Error::Api {
                status: status.as_u16(),
                body,
            });
        }
        let body = resp.text().await.map_err(|e| Error::Api {
            status: status.as_u16(),
            body: format!("failed to read body: {e}"),
        })?;
        serde_json::from_str(&body).map_err(|e| {
            tracing::error!(%e, "failed to deserialize API response");
            Error::Api {
                status: status.as_u16(),
                body: format!("JSON parse error ({e}); raw body: {body}"),
            }
        })
    }
}
