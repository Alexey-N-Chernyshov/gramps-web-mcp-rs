#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Config(#[from] crate::config::Error),
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("server task panicked: {0}")]
    Join(#[from] tokio::task::JoinError),
    #[error(transparent)]
    ServerInit(#[from] Box<rmcp::service::ServerInitializeError>),
}
