use thiserror::Error;

pub type Result<T> = std::result::Result<T, DashError>;

#[derive(Error, Debug)]
pub enum DashError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Argument error: {0}")]
    Argument(String),

    #[error("Strava API error: {0}")]
    Strava(#[from] strava::errors::StravaError),
}
