use crate::config::Config;
use crate::errors::{DashError, Result};
use std::path::PathBuf;

/// Run the interactive setup + OAuth authorization flow.
///
/// If client_id / client_secret are missing or still placeholders, prompts the
/// user to enter them. Then runs the browser-based OAuth flow to obtain a
/// refresh token. Saves the resulting config (with all defaults) to disk.
pub fn run(config_path: Option<&PathBuf>) -> Result<()> {
  let mut config = match config_path {
                     Some(path) => Config::load_from_for_auth(path),
                     None => Config::load_for_auth(),
                   }.map_err(DashError::Config)?;

  if !config.strava.has_credentials() {
    eprintln!("Strava Dashboard - First-time setup");
    eprintln!("====================================");
    eprintln!();
    eprintln!("Create a Strava API application at: https://www.strava.com/settings/api");
    eprintln!("Set the \"Authorization Callback Domain\" to: http://localhost");
    eprintln!();

    let client_id = prompt("Client ID: ")?;
    let client_secret = prompt("Client Secret: ")?;

    config.strava.set_client_id(client_id);
    config.strava.set_client_secret(client_secret);
  }

  let token_response = strava::oauth::run_auth_flow(&config.strava).map_err(DashError::Strava)?;

  config.strava.set_refresh_token(token_response.refresh_token);
  config.save().map_err(DashError::Config)?;

  eprintln!();
  eprintln!("Authorization successful! Config saved.");
  Ok(())
}

fn prompt(label: &str) -> Result<String> {
  use std::io::Write;
  eprint!("{label}");
  std::io::stderr().flush().ok();
  let mut buf = String::new();
  std::io::stdin().read_line(&mut buf)
                  .map_err(|e| DashError::Config(format!("Failed to read input: {e}")))?;
  let value = buf.trim().to_string();
  if value.is_empty() {
    return Err(DashError::Config("Input cannot be empty".to_string()));
  }
  Ok(value)
}
