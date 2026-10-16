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

use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use url::Url;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("missing or invalid environment variable: {0}")]
    Env(#[from] envy::Error),
    #[error("{0}")]
    Invalid(String),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransportMode {
    #[default]
    Stdio,
    Http,
}

fn default_http_host() -> String {
    "0.0.0.0".to_string()
}

fn default_http_port() -> u16 {
    3000
}

fn default_mcp_keep_alive() -> u64 {
    300
}

fn default_max_media_bytes() -> u64 {
    50 * 1024 * 1024
}

fn default_oauth_storage_path() -> String {
    "/data/oauth_store.json".to_string()
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub gramps_api_url: Url,
    #[serde(default)]
    pub gramps_username: Option<String>,
    #[serde(default)]
    pub gramps_password: Option<SecretString>,
    #[serde(default)]
    pub gramps_readonly: bool,
    #[serde(default)]
    pub mcp_transport: TransportMode,
    #[serde(default = "default_http_host")]
    pub mcp_http_host: String,
    #[serde(default = "default_http_port")]
    pub mcp_http_port: u16,
    pub mcp_auth_token: Option<SecretString>,
    #[serde(default)]
    pub mcp_allowed_hosts: Option<String>,
    #[serde(default = "default_mcp_keep_alive")]
    pub mcp_keep_alive: u64,
    #[serde(default = "default_max_media_bytes")]
    pub mcp_max_media_bytes: u64,
    /// Enables per-user OAuth 2.1 authorization on the HTTP transport instead of
    /// the single static `mcp_auth_token`/service-account identity.
    /// Off by default.
    #[serde(default)]
    pub mcp_oauth_enabled: bool,
    #[serde(default = "default_oauth_storage_path")]
    pub mcp_oauth_storage_path: String,
    #[serde(default)]
    pub mcp_oauth_storage_key: Option<SecretString>,
    #[serde(default)]
    pub mcp_oauth_public_url: Option<Url>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let mut cfg = envy::from_env::<Config>()?;
        // Treat MCP_AUTH_TOKEN="" the same as unset
        if cfg
            .mcp_auth_token
            .as_ref()
            .is_some_and(|t| t.expose_secret().is_empty())
        {
            cfg.mcp_auth_token = None;
        }
        cfg.validate()?;
        Ok(cfg)
    }

    fn validate(&self) -> Result<()> {
        if self.mcp_oauth_enabled {
            if self.mcp_oauth_storage_key.is_none() {
                return Err(Error::Invalid(
                    "MCP_OAUTH_STORAGE_KEY is required when MCP_OAUTH_ENABLED=true".into(),
                ));
            }
            let Some(public_url) = &self.mcp_oauth_public_url else {
                return Err(Error::Invalid(
                    "MCP_OAUTH_PUBLIC_URL is required when MCP_OAUTH_ENABLED=true".into(),
                ));
            };
            if public_url.scheme() != "https" {
                return Err(Error::Invalid(
                    "MCP_OAUTH_PUBLIC_URL must be an https:// URL".into(),
                ));
            }
        } else if self.gramps_username.is_none() || self.gramps_password.is_none() {
            return Err(Error::Invalid(
                "GRAMPS_USERNAME and GRAMPS_PASSWORD are required unless MCP_OAUTH_ENABLED=true"
                    .into(),
            ));
        }
        Ok(())
    }
}

pub fn parse_allowed_hosts(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_allowed_hosts_trims_and_filters() {
        let result = parse_allowed_hosts("a:1, b:2 ,c");
        assert_eq!(result, vec!["a:1", "b:2", "c"]);
    }

    #[test]
    fn parse_allowed_hosts_empty_string() {
        assert!(parse_allowed_hosts("").is_empty());
    }

    #[test]
    fn parse_allowed_hosts_only_commas() {
        assert!(parse_allowed_hosts(", ,").is_empty());
    }
}
