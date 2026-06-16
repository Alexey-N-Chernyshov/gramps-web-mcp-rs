use serde::Deserialize;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("missing or invalid environment variable: {0}")]
    Env(#[from] envy::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub gramps_api_url: String,
    pub gramps_username: String,
    pub gramps_password: String,
    #[serde(default)]
    pub readonly: bool,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(envy::from_env::<Config>()?)
    }
}
