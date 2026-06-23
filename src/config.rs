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

use serde::Deserialize;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("missing or invalid environment variable: {0}")]
    Env(#[from] envy::Error),
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

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub gramps_api_url: String,
    pub gramps_username: String,
    pub gramps_password: String,
    #[serde(default)]
    pub gramps_readonly: bool,
    #[serde(default)]
    pub mcp_transport: TransportMode,
    #[serde(default = "default_http_host")]
    pub mcp_http_host: String,
    #[serde(default = "default_http_port")]
    pub mcp_http_port: u16,
    pub mcp_auth_token: Option<String>,
    #[serde(default)]
    pub mcp_allowed_hosts: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let mut cfg = envy::from_env::<Config>()?;
        // Treat MCP_AUTH_TOKEN="" the same as unset
        if cfg.mcp_auth_token.as_deref() == Some("") {
            cfg.mcp_auth_token = None;
        }
        Ok(cfg)
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
