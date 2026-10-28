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
    /// A Gramps Web access token to sign in with instead of
    /// `gramps_username`/`gramps_password` (e.g. SSO-only instances).
    /// Mutually exclusive with them; unlike them, it's never refreshed.
    #[serde(default)]
    pub gramps_access_token: Option<SecretString>,
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
    #[serde(default)]
    pub mcp_allowed_origins: Option<String>,
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
        if cfg
            .gramps_access_token
            .as_ref()
            .is_some_and(|t| t.expose_secret().is_empty())
        {
            cfg.gramps_access_token = None;
        }
        cfg.validate()?;
        Ok(cfg)
    }

    fn validate(&self) -> Result<()> {
        if let Some(raw) = &self.mcp_allowed_origins {
            parse_allowed_origins(raw)?;
        }

        // Exactly one of three auth methods: OAuth, a static access token, or
        // username+password. `wants_password_login` counts intent (either
        // field set), so a lone GRAMPS_USERNAME is still caught below as an
        // incomplete password login rather than silently passing here.
        let wants_password_login = self.gramps_username.is_some() || self.gramps_password.is_some();
        let methods_chosen = [
            self.mcp_oauth_enabled,
            self.gramps_access_token.is_some(),
            wants_password_login,
        ]
        .into_iter()
        .filter(|chosen| *chosen)
        .count();
        if methods_chosen != 1 {
            return Err(Error::Invalid(
                "exactly one of MCP_OAUTH_ENABLED=true, GRAMPS_ACCESS_TOKEN, or \
                 GRAMPS_USERNAME+GRAMPS_PASSWORD is required"
                    .into(),
            ));
        }

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
        } else if wants_password_login
            && (self.gramps_username.is_none() || self.gramps_password.is_none())
        {
            return Err(Error::Invalid(
                "GRAMPS_USERNAME and GRAMPS_PASSWORD must both be set".into(),
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

/// Parses a comma-separated `MCP_ALLOWED_ORIGINS` value for
/// `StreamableHttpServerConfig::with_allowed_origins` and our own CORS
/// layer. Each entry must carry a scheme (`http://`/`https://`) — an entry
/// without one would simply never match a browser's `Origin` header, so we
/// reject it at startup instead of silently ignoring it. An entry may end
/// in `:*` to match that scheme and host on any port.
pub fn parse_allowed_origins(raw: &str) -> Result<Vec<String>> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            if s.starts_with("http://") || s.starts_with("https://") {
                Ok(s.to_owned())
            } else {
                Err(Error::Invalid(format!(
                    "invalid MCP_ALLOWED_ORIGINS entry {s:?}: must start with http:// or https://"
                )))
            }
        })
        .collect()
}

/// Matches a browser `Origin` header against a list parsed by
/// [`parse_allowed_origins`]. An entry ending in `:*` matches its
/// scheme+host on any port (including none); any other entry must equal
/// the origin exactly.
pub fn origin_matches(origin: &str, allowed: &[String]) -> bool {
    allowed.iter().any(|entry| match entry.strip_suffix(":*") {
        Some(prefix) => {
            origin == prefix
                || origin
                    .strip_prefix(prefix)
                    .is_some_and(|rest| rest.starts_with(':'))
        }
        None => origin == entry,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_config() -> Config {
        Config {
            gramps_api_url: "http://gramps.invalid".parse().unwrap(),
            gramps_username: None,
            gramps_password: None,
            gramps_access_token: None,
            gramps_readonly: false,
            mcp_transport: TransportMode::Stdio,
            mcp_http_host: default_http_host(),
            mcp_http_port: default_http_port(),
            mcp_auth_token: None,
            mcp_allowed_hosts: None,
            mcp_allowed_origins: None,
            mcp_keep_alive: default_mcp_keep_alive(),
            mcp_max_media_bytes: default_max_media_bytes(),
            mcp_oauth_enabled: false,
            mcp_oauth_storage_path: default_oauth_storage_path(),
            mcp_oauth_storage_key: None,
            mcp_oauth_public_url: None,
        }
    }

    #[test]
    fn validate_rejects_no_credentials() {
        let err = base_config().validate().unwrap_err();
        assert!(matches!(err, Error::Invalid(_)));
    }

    #[test]
    fn validate_accepts_username_and_password() {
        let config = Config {
            gramps_username: Some("svc".to_string()),
            gramps_password: Some("pw".to_string().into()),
            ..base_config()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_accepts_access_token_alone() {
        let config = Config {
            gramps_access_token: Some("token".to_string().into()),
            ..base_config()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn validate_rejects_access_token_combined_with_password_login() {
        let config = Config {
            gramps_access_token: Some("token".to_string().into()),
            gramps_username: Some("svc".to_string()),
            gramps_password: Some("pw".to_string().into()),
            ..base_config()
        };
        let err = config.validate().unwrap_err();
        assert!(matches!(err, Error::Invalid(_)));
    }

    #[test]
    fn validate_rejects_oauth_combined_with_access_token() {
        let config = Config {
            mcp_oauth_enabled: true,
            gramps_access_token: Some("token".to_string().into()),
            ..base_config()
        };
        let err = config.validate().unwrap_err();
        assert!(matches!(err, Error::Invalid(_)));
    }

    #[test]
    fn validate_rejects_oauth_combined_with_password_login() {
        let config = Config {
            mcp_oauth_enabled: true,
            gramps_username: Some("svc".to_string()),
            gramps_password: Some("pw".to_string().into()),
            ..base_config()
        };
        let err = config.validate().unwrap_err();
        assert!(matches!(err, Error::Invalid(_)));
    }

    fn oauth_config() -> Config {
        Config {
            mcp_oauth_enabled: true,
            mcp_oauth_storage_key: Some("key".to_string().into()),
            mcp_oauth_public_url: Some("https://mcp.invalid".parse().unwrap()),
            ..base_config()
        }
    }

    #[test]
    fn validate_accepts_oauth_alone() {
        assert!(oauth_config().validate().is_ok());
    }

    #[test]
    fn validate_rejects_oauth_without_storage_key() {
        let config = Config {
            mcp_oauth_storage_key: None,
            ..oauth_config()
        };
        let err = config.validate().unwrap_err();
        assert!(matches!(err, Error::Invalid(_)));
    }

    #[test]
    fn validate_rejects_oauth_without_public_url() {
        let config = Config {
            mcp_oauth_public_url: None,
            ..oauth_config()
        };
        let err = config.validate().unwrap_err();
        assert!(matches!(err, Error::Invalid(_)));
    }

    #[test]
    fn validate_rejects_oauth_with_non_https_public_url() {
        let config = Config {
            mcp_oauth_public_url: Some("http://mcp.invalid".parse().unwrap()),
            ..oauth_config()
        };
        let err = config.validate().unwrap_err();
        assert!(matches!(err, Error::Invalid(_)));
    }

    #[test]
    fn validate_rejects_username_without_password() {
        let config = Config {
            gramps_username: Some("svc".to_string()),
            ..base_config()
        };
        assert!(config.validate().is_err());
    }

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

    #[test]
    fn parse_allowed_origins_trims_and_filters() {
        let result =
            parse_allowed_origins("https://app.example.com, http://localhost:5173 ,https://x.io")
                .unwrap();
        assert_eq!(
            result,
            vec![
                "https://app.example.com",
                "http://localhost:5173",
                "https://x.io"
            ]
        );
    }

    #[test]
    fn parse_allowed_origins_empty_string() {
        assert!(parse_allowed_origins("").unwrap().is_empty());
    }

    #[test]
    fn parse_allowed_origins_only_commas() {
        assert!(parse_allowed_origins(", ,").unwrap().is_empty());
    }

    #[test]
    fn parse_allowed_origins_keeps_port_wildcard() {
        let result = parse_allowed_origins("http://localhost:*").unwrap();
        assert_eq!(result, vec!["http://localhost:*"]);
    }

    #[test]
    fn parse_allowed_origins_rejects_entry_without_scheme() {
        let err = parse_allowed_origins("example.com").unwrap_err();
        assert!(matches!(err, Error::Invalid(_)));
    }

    #[test]
    fn parse_allowed_origins_rejects_bare_host_among_valid_entries() {
        let err = parse_allowed_origins("https://good.example, localhost").unwrap_err();
        assert!(matches!(err, Error::Invalid(_)));
    }

    #[test]
    fn origin_matches_exact_entry() {
        let allowed = vec!["https://app.example.com".to_string()];
        assert!(origin_matches("https://app.example.com", &allowed));
        assert!(!origin_matches("https://app.example.com:8443", &allowed));
        assert!(!origin_matches("http://app.example.com", &allowed));
        assert!(!origin_matches("https://evil.example", &allowed));
    }

    #[test]
    fn origin_matches_port_wildcard() {
        let allowed = vec!["http://localhost:*".to_string()];
        assert!(origin_matches("http://localhost", &allowed));
        assert!(origin_matches("http://localhost:5173", &allowed));
        assert!(!origin_matches("https://localhost:5173", &allowed));
        assert!(!origin_matches("http://localhostevil.com", &allowed));
    }

    #[test]
    fn origin_matches_empty_allow_list_matches_nothing() {
        assert!(!origin_matches("https://app.example.com", &[]));
    }
}
