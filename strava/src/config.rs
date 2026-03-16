use crate::types::SportType;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const CONFIG_EXAMPLE: &str = include_str!("../../config.example.toml");

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
  /// Strava API credentials
  #[serde(default)]
  pub strava: StravaConfig,

  /// Display and dashboard settings (optional section)
  #[serde(default)]
  pub display: DisplayConfig,
}

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

/// A single sport distance goal for the dashboard progress bars.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GoalConfig {
  pub sport: SportType,
  pub km:    f64,
}

/// Display and scheduling configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DisplayConfig {
  /// Sleep interval between refreshes in seconds (default: 10800 = 3 hours)
  #[serde(default = "default_sleep_interval")]
  pub sleep_interval_secs: u64,

  /// Hour (0–23, local time) when the quiet period starts — no fetching/refresh
  /// (default: 20)
  #[serde(default = "default_quiet_start")]
  pub quiet_start_hour: u32,

  /// Hour (0–23, local time) when the quiet period ends (default: 8)
  #[serde(default = "default_quiet_end")]
  pub quiet_end_hour: u32,

  /// Ordered sport goals (1–3). Controls which progress bars appear and their
  /// order. First goal is always full-width; with 3 goals, 2nd and 3rd share
  /// a row.
  #[serde(default = "default_goals")]
  pub goals: Vec<GoalConfig>,
}

fn default_sleep_interval() -> u64 {
  10800
}
fn default_quiet_start() -> u32 {
  20
}
fn default_quiet_end() -> u32 {
  8
}
fn default_goals() -> Vec<GoalConfig> {
  vec![GoalConfig { sport: SportType::Run,
                    km:    800.0, },
       GoalConfig { sport: SportType::Ride,
                    km:    5000.0, },
       GoalConfig { sport: SportType::Swim,
                    km:    30.0, },]
}

impl Default for DisplayConfig {
  fn default() -> Self {
    Self { sleep_interval_secs: default_sleep_interval(),
           quiet_start_hour:    default_quiet_start(),
           quiet_end_hour:      default_quiet_end(),
           goals:               default_goals(), }
  }
}

impl DisplayConfig {
  /// Look up the goal distance for a sport, if configured.
  pub fn goal_for(&self, sport: SportType) -> Option<f64> {
    self.goals.iter().find(|g| g.sport == sport).map(|g| g.km)
  }
}

impl Config {
  /// Default config directory: ~/.config/rpi-zero2w-strava-dash/
  fn config_dir() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from(".config"))
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

      if let Err(e) = fs::write(&path, CONFIG_EXAMPLE) {
        return Err(format!("Failed to write config template: {e}"));
      }

      return Err(format!("Config file not found. A template has been created at: {}\nPlease fill in your Strava API credentials.",
                         path.display()));
    }

    Self::load_from(&path)
  }

  /// Load config from an explicit file path.
  pub fn load_from(path: &Path) -> Result<Self, String> {
    let contents = fs::read_to_string(path).map_err(|e| {
                                             format!("Failed to read config file {}: {e}",
                                                     path.display())
                                           })?;

    let config: Config = toml::from_str(&contents).map_err(|e| {
                                                    format!("Failed to parse config file {}: {e}",
                                                            path.display())
                                                  })?;

    if config.strava.client_id == "YOUR_CLIENT_ID"
       || config.strava.client_secret == "YOUR_CLIENT_SECRET"
       || config.strava.refresh_token == "YOUR_REFRESH_TOKEN"
       || config.strava.client_id.is_empty()
    {
      return Err(format!("Please fill in your Strava API credentials in: {}", path.display()));
    }

    log::info!("Loaded config from {}", path.display());
    Ok(config)
  }

  pub fn client_id(&self) -> &str {
    &self.strava.client_id
  }

  pub fn client_secret(&self) -> &str {
    &self.strava.client_secret
  }

  pub fn refresh_token(&self) -> &str {
    &self.strava.refresh_token
  }

  pub fn set_refresh_token(&mut self, token: String) {
    self.strava.refresh_token = token;
  }

  /// Save the current config back to disk.
  pub fn save(&self) -> Result<(), String> {
    let toml_string =
      toml::to_string_pretty(self).map_err(|e| format!("Failed to serialize config: {e}"))?;

    let contents = format!("# Strava API credentials\n# See docs/strava.md for setup instructions\n\n{toml_string}");

    fs::write(Self::config_path(), contents).map_err(|e| format!("Failed to write config: {e}"))?;

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

    Self::load_from_for_auth(&path)
  }

  /// Load config for auth from an explicit file path.
  pub fn load_from_for_auth(path: &Path) -> Result<Self, String> {
    let contents = fs::read_to_string(path).map_err(|e| {
                                             format!("Failed to read config file {}: {e}",
                                                     path.display())
                                           })?;

    let config: Config = toml::from_str(&contents).map_err(|e| {
                                                    format!("Failed to parse config file {}: {e}",
                                                            path.display())
                                                  })?;

    if config.strava.client_id == "YOUR_CLIENT_ID"
       || config.strava.client_secret == "YOUR_CLIENT_SECRET"
       || config.strava.client_id.is_empty()
    {
      return Err(format!("Please fill in your client_id and client_secret in: {}",
                         path.display()));
    }

    log::info!("Loaded config (for auth) from {}", path.display());
    Ok(config)
  }

  /// Parse a Config from a TOML string (no file I/O).
  pub fn from_toml(toml_str: &str) -> Result<Self, String> {
    toml::from_str(toml_str).map_err(|e| format!("Failed to parse TOML: {e}"))
  }

  /// Serialize the Config to a TOML string.
  pub fn to_toml(&self) -> Result<String, String> {
    let toml_string =
      toml::to_string_pretty(self).map_err(|e| format!("Failed to serialize config: {e}"))?;
    Ok(format!("# Strava API credentials\n# See docs/strava.md for setup instructions\n\n{toml_string}"))
  }
}
