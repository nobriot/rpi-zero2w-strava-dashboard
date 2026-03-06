use thiserror::Error;

pub type Result<T> = std::result::Result<T, DashError>;

#[derive(Error, Debug)]
pub enum DashError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Strava API error: {0}")]
    Strava(String),
}
