use thiserror::Error;

#[derive(Error, Debug)]
pub enum StravaError {
    #[error("Strava API response error: {0}")]
    StravaApiResponseError(String),

    #[error("Strava API response missing body")]
    StravaApiResponseMissingBody,

    #[error("Strava API de-serialization error. Problematic data: {0}")]
    StravaApiResponseDeserializationError(String),

    #[error("Unauthorized (HTTP 401)")]
    Unauthorized,

    #[error("OAuth flow error: {0}")]
    OAuthError(String),

    #[error("Network unavailable: {0}")]
    NetworkUnavailable(String),
}
