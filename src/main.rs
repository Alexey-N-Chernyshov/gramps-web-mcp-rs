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

use gramps_web_mcp_rs::{app::build_app, app_error::AppError, config, server};
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
        config::TransportMode::Stdio => run_stdio(config).await,
        config::TransportMode::Http => run_http(config).await,
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
    let bind_addr = format!("{}:{}", config.mcp_http_host, config.mcp_http_port);

    let app = build_app(config).await?;

    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;

    tracing::info!("gramps-web-mcp-rs started on http://{bind_addr}/mcp");

    axum::serve(listener, app).await?;

    Ok(())
}
