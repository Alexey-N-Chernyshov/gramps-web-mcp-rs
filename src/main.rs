mod app_error;
mod auth;
mod client;
mod config;
mod models;
mod server;
mod tools;

use app_error::AppError;
use rmcp::{transport::io::stdio, ServiceExt};

#[tokio::main]
async fn main() -> Result<(), AppError> {
    init_tracing();

    let config = config::Config::from_env()?;

    tracing::info!(
        "Starting gramps-mcp-rs, API base: {}",
        config.gramps_api_url
    );

    run(config).await
}

fn init_tracing() {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(std::io::stderr))
        .with(EnvFilter::from_default_env())
        .init();
}

async fn run(config: config::Config) -> Result<(), AppError> {
    let mcp_server = server::GrampsMcpServer::new(config)?;
    let transport = stdio();
    let server = mcp_server
        .serve(transport)
        .await
        .map_err(|e| AppError::ServerInit(Box::new(e)))?;

    tracing::info!("gramps-mcp-rs started on stdio");

    server.waiting().await?;

    Ok(())
}
