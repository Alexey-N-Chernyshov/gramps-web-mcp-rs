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

mod app_error;
mod auth;
mod client;
mod config;
mod models;
mod server;
mod tools;

use std::sync::Arc;

use app_error::AppError;
use config::TransportMode;
use rmcp::{transport::io::stdio, ServiceExt};

#[tokio::main]
async fn main() -> Result<(), AppError> {
    init_tracing();

    let config = config::Config::from_env()?;

    tracing::info!(
        "Starting gramps-web-mcp-rs, API base: {}",
        config.gramps_api_url
    );

    match config.mcp_transport.clone() {
        TransportMode::Stdio => run_stdio(config).await,
        TransportMode::Http => run_http(config).await,
    }
}

fn init_tracing() {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(std::io::stderr))
        .with(EnvFilter::from_default_env())
        .init();
}

async fn run_stdio(config: config::Config) -> Result<(), AppError> {
    let mcp_server = server::GrampsMcpServer::new(config)?;
    let transport = stdio();
    let server = mcp_server
        .serve(transport)
        .await
        .map_err(|e| AppError::ServerInit(Box::new(e)))?;

    tracing::info!("gramps-web-mcp-rs started on stdio");

    server.waiting().await?;

    Ok(())
}

async fn run_http(config: config::Config) -> Result<(), AppError> {
    use axum::http::Method;
    use rmcp::transport::streamable_http_server::{
        session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
    };
    use tower_http::cors::CorsLayer;

    let host = config.mcp_http_host.clone();
    let port = config.mcp_http_port;
    let bind_addr = format!("{host}:{port}");
    let auth_token = config.mcp_auth_token.clone();
    let allowed_hosts_raw = config.mcp_allowed_hosts.clone();

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

    let session_manager = Arc::new(LocalSessionManager::default());
    let service =
        StreamableHttpService::new(move || Ok(mcp_server.clone()), session_manager, http_config);

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::DELETE])
        .allow_headers(tower_http::cors::Any)
        .allow_origin(tower_http::cors::Any);

    let mcp_router = match auth_token {
        Some(token) => {
            tracing::info!("HTTP auth enabled");
            axum::Router::new()
                .nest_service("/mcp", service)
                .route_layer(axum::middleware::from_fn_with_state(
                    token,
                    bearer_auth_middleware,
                ))
        }
        None => {
            tracing::warn!("HTTP auth disabled — set MCP_AUTH_TOKEN for public deployments");
            axum::Router::new().nest_service("/mcp", service)
        }
    };

    let app = mcp_router
        .route("/health", axum::routing::get(|| async { "ok" }))
        .layer(cors);

    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;

    tracing::info!("gramps-web-mcp-rs started on http://{bind_addr}/mcp");

    axum::serve(listener, app).await?;

    Ok(())
}

async fn bearer_auth_middleware(
    axum::extract::State(token): axum::extract::State<String>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    let expected = format!("Bearer {token}");
    let authorized = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(|v| v == expected)
        .unwrap_or(false);

    if authorized {
        next.run(req).await
    } else {
        (StatusCode::UNAUTHORIZED, "Unauthorized").into_response()
    }
}
