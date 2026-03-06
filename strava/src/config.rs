use serde::Deserialize;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    client_id: String,
    client_secret: String,
    refresh_token: String,
}

impl Config {
    /// Default config directory: ~/.config/rpi-zero2w-strava-dash/
    fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from(".config"))
            .join("rpi-zero2w-strava-dash")
    }

    fn config_path() -> PathBuf {
        Self::config_dir().join("config.toml")
    }

    /// Load config from ~/.config/rpi-zero2w-strava-dash/config.toml
    /// Creates a template file and returns an error if it doesn't exist yet.
    pub fn load() -> Result<Self, String> {
        let path = Self::config_path();

        if !path.exists() {
            // Create the directory and a template file
            if let Err(e) = fs::create_dir_all(Self::config_dir()) {
                return Err(format!("Failed to create config directory: {e}"));
            }

            let template = r#"# Strava API credentials
# See docs/strava.md for setup instructions
client_id = "YOUR_CLIENT_ID"
client_secret = "YOUR_CLIENT_SECRET"
refresh_token = "YOUR_REFRESH_TOKEN"
"#;
            if let Err(e) = fs::write(&path, template) {
                return Err(format!("Failed to write config template: {e}"));
            }

            return Err(format!(
                "Config file not found. A template has been created at: {}\nPlease fill in your Strava API credentials.",
                path.display()
            ));
        }

        let contents = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read config file {}: {e}", path.display()))?;

        let config: Config = toml::from_str(&contents)
            .map_err(|e| format!("Failed to parse config file {}: {e}", path.display()))?;

        if config.client_id == "YOUR_CLIENT_ID"
            || config.client_secret == "YOUR_CLIENT_SECRET"
            || config.refresh_token == "YOUR_REFRESH_TOKEN"
        {
            return Err(format!(
                "Please fill in your Strava API credentials in: {}",
                path.display()
            ));
        }

        log::info!("Loaded config from {}", path.display());
        Ok(config)
    }

    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    pub fn client_secret(&self) -> &str {
        &self.client_secret
    }

    pub fn refresh_token(&self) -> &str {
        &self.refresh_token
    }
}
