use serde::{Deserialize, Serialize};

/// Strava API credentials.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct StravaConfig {
  #[serde(default)]
  client_id:     String,
  #[serde(default)]
  client_secret: String,
  #[serde(default)]
  refresh_token: String,
}

impl StravaConfig {
  pub fn client_id(&self) -> &str {
    &self.client_id
  }

  pub fn client_secret(&self) -> &str {
    &self.client_secret
  }

  pub fn refresh_token(&self) -> &str {
    &self.refresh_token
  }

  pub fn set_refresh_token(&mut self, token: String) {
    self.refresh_token = token;
  }

  pub fn set_client_id(&mut self, id: String) {
    self.client_id = id;
  }

  pub fn set_client_secret(&mut self, secret: String) {
    self.client_secret = secret;
  }

  /// Returns true if client_id and client_secret are filled in with real
  /// values.
  pub fn has_credentials(&self) -> bool {
    !self.client_id.is_empty()
    && self.client_id != "YOUR_CLIENT_ID"
    && !self.client_secret.is_empty()
    && self.client_secret != "YOUR_CLIENT_SECRET"
  }
}
