use serde::Deserialize;
use serde::Serialize;

/// Expected response body for the Access Token endpoint
#[derive(Debug, Deserialize, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
}
