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

use std::sync::Arc;

use axum::http::Method;
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
};
use secrecy::ExposeSecret;
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::{app_error::AppError, config::Config, http_auth, oauth, server};

/// Builds the HTTP transport's full router: the MCP service nested at `/mcp` behind whichever
/// auth mode `config` selects (per-user OAuth, a static bearer token, or none), `/health`, CORS
/// (only when `MCP_ALLOWED_ORIGINS` is set), and request tracing. Doesn't bind a listener or
/// serve — that's `main.rs`'s job — so tests can exercise this exact router without a real
/// Gramps Web backend behind it (constructing the server does no network I/O).
pub async fn build_app(config: Config) -> Result<axum::Router, AppError> {
    let auth_token = config.mcp_auth_token.clone();
    let allowed_hosts_raw = config.mcp_allowed_hosts.clone();
    let allowed_origins = match config.mcp_allowed_origins.as_deref() {
        Some(raw) => crate::config::parse_allowed_origins(raw)?,
        None => Vec::new(),
    };
    let oauth_enabled = config.mcp_oauth_enabled;
    let oauth_storage_path = config.mcp_oauth_storage_path.clone();
    let oauth_storage_key = config.mcp_oauth_storage_key.clone();
    let oauth_public_url = config.mcp_oauth_public_url.clone();
    let gramps_api_url = config.gramps_api_url.clone();
    let max_media_bytes = config.mcp_max_media_bytes;
    let keep_alive = match config.mcp_keep_alive {
        0 => {
            tracing::info!("HTTP session keep-alive disabled");
            None
        }
        secs => {
            tracing::info!("HTTP session keep-alive: {} seconds", secs);
            Some(std::time::Duration::from_secs(secs))
        }
    };

    let mcp_server = server::GrampsMcpServer::new(config)?;

    let http_config = match allowed_hosts_raw.as_deref() {
        Some(raw) => {
            let hosts = crate::config::parse_allowed_hosts(raw);
            if hosts.is_empty() {
                StreamableHttpServerConfig::default()
            } else {
                tracing::info!("HTTP allowed hosts: {:?}", hosts);
                StreamableHttpServerConfig::default().with_allowed_hosts(hosts)
            }
        }
        None => StreamableHttpServerConfig::default(),
    };
    let http_config = if allowed_origins.is_empty() {
        tracing::info!(
            "HTTP Origin validation enforced: requests carrying an Origin header are rejected \
             (403) unless MCP_ALLOWED_ORIGINS lists it; requests with no Origin header still \
             pass through"
        );
        http_config.enforce_origin_validation()
    } else {
        tracing::info!("HTTP allowed origins: {:?}", allowed_origins);
        http_config.with_allowed_origins(allowed_origins.clone())
    }
    // Plain JSON responses instead of chunked SSE for simple request/response
    // calls — some reverse proxies (and flaky tunnels) mishandle chunked
    // text/event-stream bodies.
    .with_json_response(true);

    let mut session_manager = LocalSessionManager::default();
    session_manager.session_config.keep_alive = keep_alive;
    let session_manager = Arc::new(session_manager);

    let service =
        StreamableHttpService::new(move || Ok(mcp_server.clone()), session_manager, http_config);

    let mcp_router = if oauth_enabled {
        tracing::info!("OAuth mode enabled — per-user Gramps identity on every request");
        let store = oauth::Store::open(
            oauth_storage_path,
            oauth_storage_key
                .as_ref()
                .expect("validated non-empty by Config::from_env when OAuth is enabled")
                .expose_secret(),
        )
        .await?;
        let oauth_state = Arc::new(oauth::OAuthState {
            store,
            api_url: gramps_api_url,
            http: reqwest::Client::new(),
            issuer: oauth_public_url
                .expect("validated non-empty by Config::from_env when OAuth is enabled"),
            max_media_bytes,
        });
        axum::Router::new()
            .nest_service("/mcp", service)
            .route_layer(axum::middleware::from_fn_with_state(
                oauth_state.clone(),
                oauth::identity_middleware,
            ))
            .merge(oauth::router(oauth_state))
    } else {
        match auth_token {
            Some(token) => {
                tracing::info!("HTTP auth enabled");
                axum::Router::new()
                    .nest_service("/mcp", service)
                    .route_layer(axum::middleware::from_fn_with_state(
                        token,
                        http_auth::bearer_auth_middleware,
                    ))
            }
            None => {
                tracing::warn!("HTTP auth disabled — set MCP_AUTH_TOKEN for public deployments");
                axum::Router::new().nest_service("/mcp", service)
            }
        }
    };

    let cors = if allowed_origins.is_empty() {
        None
    } else {
        let origins = allowed_origins.clone();
        Some(
            CorsLayer::new()
                .allow_methods([Method::GET, Method::POST, Method::DELETE])
                .allow_headers(tower_http::cors::Any)
                .allow_origin(AllowOrigin::predicate(move |origin, _parts| {
                    origin
                        .to_str()
                        .is_ok_and(|o| crate::config::origin_matches(o, &origins))
                })),
        )
    };

    let app = mcp_router.route("/health", axum::routing::get(|| async { "ok" }));
    let app = match cors {
        Some(cors) => app.layer(cors),
        None => app,
    };
    let app = app.layer(tower_http::trace::TraceLayer::new_for_http());

    Ok(app)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TransportMode;

    fn base_config() -> Config {
        Config {
            gramps_api_url: "http://gramps.invalid"
                .parse()
                .expect("test api url should be a valid URL"),
            gramps_username: Some("svc".to_string()),
            gramps_password: Some("pw".to_string().into()),
            gramps_sync_token: None,
            gramps_readonly: false,
            mcp_transport: TransportMode::Http,
            mcp_http_host: "127.0.0.1".to_string(),
            mcp_http_port: 0,
            mcp_auth_token: None,
            mcp_allowed_hosts: None,
            mcp_allowed_origins: None,
            mcp_keep_alive: 300,
            mcp_max_media_bytes: 50 * 1024 * 1024,
            mcp_oauth_enabled: false,
            mcp_oauth_storage_path: "/tmp/unused-build-app-tests.json".to_string(),
            mcp_oauth_storage_key: None,
            mcp_oauth_public_url: None,
        }
    }

    async fn spawn(app: axum::Router) -> String {
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
        format!("http://{addr}")
    }

    async fn health_status(app: axum::Router) -> reqwest::StatusCode {
        let base_url = spawn(app).await;
        reqwest::Client::new()
            .get(format!("{base_url}/health"))
            .send()
            .await
            .expect("GET /health should succeed")
            .status()
    }

    #[tokio::test]
    async fn build_app_without_auth_serves_health() {
        let app = build_app(base_config()).await.unwrap();
        assert_eq!(health_status(app).await, reqwest::StatusCode::OK);
    }

    #[tokio::test]
    async fn build_app_with_bearer_token_serves_health() {
        let config = Config {
            mcp_auth_token: Some("secret-token".to_string().into()),
            ..base_config()
        };
        let app = build_app(config).await.unwrap();
        assert_eq!(health_status(app).await, reqwest::StatusCode::OK);
    }

    #[tokio::test]
    async fn build_app_with_oauth_serves_health() {
        let store_path = std::env::temp_dir().join(format!(
            "build-app-oauth-test-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let key = {
            use base64::Engine as _;
            base64::engine::general_purpose::STANDARD.encode([7u8; 32])
        };
        let config = Config {
            gramps_username: None,
            gramps_password: None,
            mcp_oauth_enabled: true,
            mcp_oauth_storage_path: store_path.to_string_lossy().into_owned(),
            mcp_oauth_storage_key: Some(key.into()),
            mcp_oauth_public_url: Some(
                "https://mcp.invalid"
                    .parse()
                    .expect("test issuer should be a valid URL"),
            ),
            ..base_config()
        };
        let app = build_app(config).await.unwrap();
        assert_eq!(health_status(app).await, reqwest::StatusCode::OK);

        let _ = std::fs::remove_file(&store_path);
    }
}
