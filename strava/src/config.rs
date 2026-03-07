use serde::Deserialize;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    client_id: String,
    client_secret: String,
    refresh_token: String,

    /// Display and dashboard settings (optional section)
    #[serde(default)]
    pub display: DisplayConfig,
}

/// Display and scheduling configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DisplayConfig {
    /// Sleep interval between refreshes in seconds (default: 10800 = 3 hours)
    #[serde(default = "default_sleep_interval")]
    pub sleep_interval_secs: u64,

    /// Yearly running goal in km (default: 2000)
    #[serde(default = "default_run_goal")]
    pub run_goal_km: f64,

    /// Yearly cycling goal in km (default: 5000)
    #[serde(default = "default_ride_goal")]
    pub ride_goal_km: f64,
}

fn default_sleep_interval() -> u64 {
    10800
}
fn default_run_goal() -> f64 {
    2000.0
}
fn default_ride_goal() -> f64 {
    5000.0
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            sleep_interval_secs: default_sleep_interval(),
            run_goal_km: default_run_goal(),
            ride_goal_km: default_ride_goal(),
        }
    }
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

# Display settings (all optional, shown with defaults)
# [display]
# sleep_interval_secs = 10800  # 3 hours
# run_goal_km = 2000.0
# ride_goal_km = 5000.0
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

    pub fn set_refresh_token(&mut self, token: String) {
        self.refresh_token = token;
    }

    /// Save the current config back to disk.
    pub fn save(&self) -> Result<(), String> {
        let toml_string = toml::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {e}"))?;

        let contents = format!(
            "# Strava API credentials\n# See docs/strava.md for setup instructions\n\n{toml_string}"
        );

        fs::write(Self::config_path(), contents)
            .map_err(|e| format!("Failed to write config: {e}"))?;

        log::info!("Config saved to {}", Self::config_path().display());
        Ok(())
    }

    /// Load config allowing a placeholder refresh_token (for use with --auth).
    /// Still requires valid client_id and client_secret.
    pub fn load_for_auth() -> Result<Self, String> {
        let path = Self::config_path();

        if !path.exists() {
            // Delegate to load() which creates the template
            return Self::load();
        }

        let contents = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read config file {}: {e}", path.display()))?;

        let config: Config = toml::from_str(&contents)
            .map_err(|e| format!("Failed to parse config file {}: {e}", path.display()))?;

        if config.client_id == "YOUR_CLIENT_ID" || config.client_secret == "YOUR_CLIENT_SECRET" {
            return Err(format!(
                "Please fill in your client_id and client_secret in: {}",
                path.display()
            ));
        }

        log::info!("Loaded config (for auth) from {}", path.display());
        Ok(config)
    }
}
