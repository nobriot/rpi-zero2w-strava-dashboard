use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Set file permissions to owner-only read/write (0o600) to protect
/// credentials.
#[cfg(unix)]
fn restrict_permissions(path: &Path) {
  use std::os::unix::fs::PermissionsExt;
  let perms = fs::Permissions::from_mode(0o600);
  if let Err(e) = fs::set_permissions(path, perms) {
    log::warn!("Failed to set permissions on {}: {e}", path.display());
  }
}

const CONFIG_EXAMPLE: &str = include_str!("../../dist/config.example.toml");

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
  /// Strava API credentials
  #[serde(default)]
  pub strava: strava::config::StravaConfig,

  /// Display and dashboard settings (optional section)
  #[serde(default)]
  pub display: display::config::DisplayConfig,

  /// Power management settings (optional section)
  #[serde(default)]
  pub power: PowerConfig,
}

/// Power management configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PowerConfig {
  /// Enable rtcwake poweroff between refresh cycles for maximum battery
  /// savings. Requires DS3231 RTC with INT/SQW wired to GPIO4.
  /// Default: false.
  #[serde(default)]
  pub shutdown_after_cycle: bool,

  /// Refresh interval (seconds) when charging or when no battery sensor is
  /// detected (e.g. dev machine). Default: 1200 (20 minutes).
  #[serde(default = "default_charging_interval")]
  pub charging_interval_secs: u64,

  /// Seconds to stay awake after each refresh cycle before sleeping or
  /// shutting down. Gives a window for SSH access. Default: 120 (2 minutes).
  #[serde(default = "default_linger")]
  pub linger_secs: u64,

  /// Don't rtcwake-shutdown while SSH sessions are active unless the battery
  /// percentage drops below this value. Set to 0 to disable SSH detection.
  /// Default: 30.
  #[serde(default = "default_ssh_inhibit")]
  pub ssh_inhibit_below_percent: u8,

  /// Sync /boot/firmware/config.txt with the expected version at startup.
  /// Default: false.
  #[serde(default)]
  pub sync_firmware: bool,
}

fn default_charging_interval() -> u64 {
  1200
}
fn default_linger() -> u64 {
  120
}
fn default_ssh_inhibit() -> u8 {
  30
}

impl Default for PowerConfig {
  fn default() -> Self {
    Self { shutdown_after_cycle:      false,
           charging_interval_secs:    default_charging_interval(),
           linger_secs:               default_linger(),
           ssh_inhibit_below_percent: default_ssh_inhibit(),
           sync_firmware:             false, }
  }
}

impl Config {
  /// Default config directory: ~/.config/rpi-zero2w-strava-dashboard/
  fn config_dir() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from(".config"))
                      .join(env!("CARGO_PKG_NAME"))
  }

  fn config_path() -> PathBuf {
    Self::config_dir().join("config.toml")
  }

  /// Load config from ~/.config/rpi-zero2w-strava-dashboard/config.toml
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
      #[cfg(unix)]
      restrict_permissions(&path);

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

    if config.strava.client_id() == "YOUR_CLIENT_ID"
       || config.strava.client_secret() == "YOUR_CLIENT_SECRET"
       || config.strava.refresh_token() == "YOUR_REFRESH_TOKEN"
       || config.strava.client_id().is_empty()
    {
      return Err(format!("Please fill in your Strava API credentials in: {}", path.display()));
    }

    log::info!("Loaded config from {}", path.display());
    Ok(config)
  }

  /// Save the current config back to disk.
  pub fn save(&self) -> Result<(), String> {
    let toml_string =
      toml::to_string_pretty(self).map_err(|e| format!("Failed to serialize config: {e}"))?;

    let contents = format!("# Strava API credentials\n# See docs/strava.md for setup instructions\n\n{toml_string}");

    let path = Self::config_path();
    fs::write(&path, contents).map_err(|e| format!("Failed to write config: {e}"))?;
    #[cfg(unix)]
    restrict_permissions(&path);

    log::info!("Config saved to {}", path.display());
    Ok(())
  }

  /// Load config for the auth flow. Returns the config even if credentials
  /// are missing/placeholder — the caller can prompt interactively.
  pub fn load_for_auth() -> Result<Self, String> {
    let path = Self::config_path();

    if !path.exists() {
      // No config file yet — return defaults so the interactive setup can fill it in
      log::info!("No config file found, using defaults for interactive setup");
      return Ok(Self { strava:  strava::config::StravaConfig::default(),
                       display: display::config::DisplayConfig::default(),
                       power:   PowerConfig::default(), });
    }

    Self::load_from_for_auth(&path)
  }

  /// Load config for auth from an explicit file path. Does not reject
  /// placeholder credentials — the caller handles that interactively.
  pub fn load_from_for_auth(path: &Path) -> Result<Self, String> {
    let contents = fs::read_to_string(path).map_err(|e| {
                                             format!("Failed to read config file {}: {e}",
                                                     path.display())
                                           })?;

    let config: Config = toml::from_str(&contents).map_err(|e| {
                                                    format!("Failed to parse config file {}: {e}",
                                                            path.display())
                                                  })?;

    log::info!("Loaded config (for auth) from {}", path.display());
    Ok(config)
  }
}
