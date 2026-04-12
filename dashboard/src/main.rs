mod args;
mod config;
mod ds3231;
mod errors;
mod firmware;
mod ina219;
mod logging;
mod power;
mod schedule;

use args::Args;
use chrono::{Datelike, NaiveDate, Utc};
use config::Config;
use errors::{DashError, Result};
use std::path::PathBuf;

static PROGRAM_NAME: &str = env!("CARGO_PKG_NAME");

fn main() {
  let result = run();

  match result {
    Ok(_) => {},
    Err(e) => {
      eprintln!("{} - error: {:?}", PROGRAM_NAME, e);
      std::process::exit(1);
    },
  }
}

/// Run the interactive setup + OAuth authorization flow.
///
/// If client_id / client_secret are missing or still placeholders, prompts the
/// user to enter them. Then runs the browser-based OAuth flow to obtain a
/// refresh token. Saves the resulting config (with all defaults) to disk.
fn run_auth(config_path: Option<&PathBuf>) -> Result<()> {
  let mut config = match config_path {
                     Some(path) => Config::load_from_for_auth(path),
                     None => Config::load_for_auth(),
                   }.map_err(DashError::Config)?;

  if !config.strava.has_credentials() {
    eprintln!("Strava Dashboard — First-time setup");
    eprintln!("====================================");
    eprintln!();
    eprintln!("Create a Strava API application at: https://www.strava.com/settings/api");
    eprintln!("Set the \"Authorization Callback Domain\" to: localhost");
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

/// Read a line from stdin with a prompt, trimming whitespace.
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

fn run() -> Result<()> {
  let args = Args::try_parse()?;
  logging::setup(args.log_file.as_deref());

  // Clear any pending DS3231 alarm from a previous rtcwake cycle
  ds3231::clear_alarm_if_pending();

  if args.auth {
    return run_auth(args.config.as_ref());
  }

  if args.clear_cache {
    strava::cache::Cache::new().clear().map_err(DashError::Config)?;
    eprintln!("Cache cleared.");
  }

  // Load config
  let mut config = match args.config.as_ref() {
                     Some(path) => Config::load_from(path),
                     None => Config::load(),
                   }.map_err(DashError::Config)?;
  log::info!("Config loaded successfully");

  // Sync boot firmware config if enabled via CLI flag or config file
  if args.sync_firmware || config.power.sync_firmware {
    match firmware::sync_boot_config() {
      Ok(true) => log::info!("Boot firmware config updated"),
      Ok(false) => {},
      Err(e) => log::warn!("Failed to sync boot firmware config: {e}"),
    }
  }

  let mut power_mgr = power::PowerManager::new(config.power.tpl5110_done_pin);

  loop {
    power_mgr.enable_wifi();

    // Read battery early so we can skip the cycle during quiet hours
    let battery = power::read_battery();

    if schedule::should_skip_cycle(&config.power, battery.as_ref()) {
      log::info!("Quiet hours with TPL5110 -- skipping cycle");
    } else {
      run_cycle(&mut config, &args, &mut power_mgr)?;
    }

    if args.once {
      break;
    }

    // Re-read battery (cycle may have taken a while)
    let battery = power::read_battery();
    let plan = schedule::plan(&config.power, battery.as_ref());

    match plan {
      schedule::SleepPlan::OnPower { sleep_secs } => {
        power_mgr.set_normal();
        log::info!("On power -- sleeping {sleep_secs}s before next cycle");
        std::thread::sleep(std::time::Duration::from_secs(sleep_secs));
      },
      schedule::SleepPlan::Battery { sleep_secs, linger_secs, .. } => {
        power_mgr.set_low_power();
        let lingered = linger(linger_secs);
        let remaining = sleep_secs.saturating_sub(lingered);
        let battery_pct = battery.as_ref().map(|b| b.percentage());
        if shutdown_or_sleep(&config.power, &mut power_mgr, remaining, battery_pct) {
          break;
        }
      },
    }
  }

  Ok(())
}

/// Run one dashboard cycle with error recovery (OAuth re-auth, network retry).
fn run_cycle(config: &mut Config, args: &Args, power_mgr: &mut power::PowerManager) -> Result<()> {
  match try_cycle(config, args) {
    Ok(()) => {},
    Err(DashError::Strava(strava::errors::StravaError::Unauthorized)) => {
      log::warn!("Unauthorized after auto-refresh -- attempting full OAuth re-authorization");
      eprintln!("\nRefresh token invalid. Starting OAuth authorization flow...");

      let token_response = strava::oauth::run_auth_flow(&config.strava)?;
      config.strava.set_refresh_token(token_response.refresh_token);
      config.save().map_err(DashError::Config)?;

      // Need WiFi for retry -- ensure it's on
      power_mgr.enable_wifi();
      if let Err(e) = try_cycle(config, args) {
        eprintln!("Error after re-authorization: {e:?}");
      }
    },
    Err(DashError::Strava(strava::errors::StravaError::NetworkUnavailable(ref msg))) => {
      log::warn!("Network unavailable: {msg}");
      eprintln!("Network unavailable -- will retry next cycle");
    },
    Err(e) => {
      eprintln!("Error during cycle: {e:?}");
    },
  }
  Ok(())
}

/// Wait up to `max_secs` for SSH access, polling every 60s.
/// Returns the number of seconds actually waited. Ends early if no SSH
/// sessions are detected.
fn linger(max_secs: u64) -> u64 {
  if max_secs == 0 {
    return 0;
  }
  log::info!("Lingering {max_secs}s for SSH access...");
  let mut waited = 0u64;
  while waited < max_secs {
    let chunk = 60.min(max_secs - waited);
    std::thread::sleep(std::time::Duration::from_secs(chunk));
    waited += chunk;
    if waited < max_secs && !power::has_ssh_sessions() {
      log::info!("No SSH sessions -- ending linger early");
      break;
    }
  }
  waited
}

/// Try to shut down (TPL5110 -> rtcwake -> software sleep).
/// Respects SSH inhibition: if an SSH session is active and battery is
/// above the inhibit threshold, polls until sessions end before shutting
/// down. Returns `true` if the caller should exit the main loop (hard
/// shutdown initiated).
fn shutdown_or_sleep(power: &config::PowerConfig,
                     power_mgr: &mut power::PowerManager,
                     remaining: u64,
                     battery_pct: Option<u8>)
                     -> bool {
  let ssh_active = power::has_ssh_sessions();
  let ssh_inhibits = power.ssh_inhibit_below_percent > 0
                     && ssh_active
                     && battery_pct.is_none_or(|p| p > power.ssh_inhibit_below_percent);

  log::info!("Power: battery={}, ssh={ssh_active}, ssh_inhibits={ssh_inhibits}, \
              shutdown_after_cycle={}",
             battery_pct.map_or("N/A".to_string(), |p| format!("{p}%")),
             power.shutdown_after_cycle);

  if ssh_inhibits {
    return poll_ssh_then_shutdown(power, power_mgr, remaining);
  }

  try_shutdown(power, power_mgr);

  // Still alive -- disable WiFi and software-sleep the remaining time
  power_mgr.disable_wifi();
  if remaining > 0 {
    std::thread::sleep(std::time::Duration::from_secs(remaining));
  }
  false
}

/// Poll every 60s until SSH sessions end, then shut down.
fn poll_ssh_then_shutdown(power: &config::PowerConfig,
                          power_mgr: &mut power::PowerManager,
                          remaining: u64)
                          -> bool {
  log::info!("SSH inhibiting shutdown -- polling every 60s");
  let mut waited = 0u64;
  while waited < remaining {
    let chunk = 60.min(remaining - waited);
    std::thread::sleep(std::time::Duration::from_secs(chunk));
    waited += chunk;
    if !power::has_ssh_sessions() {
      log::info!("SSH sessions ended -- proceeding to sleep");
      power_mgr.disable_wifi();
      let left = remaining.saturating_sub(waited);
      if try_shutdown(power, power_mgr) {
        return true;
      }
      if left > 0 {
        std::thread::sleep(std::time::Duration::from_secs(left));
      }
      break;
    }
  }
  false
}

/// Attempt hard shutdown (TPL5110 DONE signal + shutdown -h now).
/// Returns `true` if shutdown was initiated (caller should exit).
fn try_shutdown(power: &config::PowerConfig, power_mgr: &mut power::PowerManager) -> bool {
  if power.shutdown_after_cycle || power.tpl5110_done_pin.is_some() {
    return power_mgr.shutdown();
  }
  false
}

/// Run one full cycle: fetch stats -> render image -> display (or save PNG).
fn try_cycle(config: &mut Config, args: &Args) -> Result<()> {
  let (stats, avatar, is_offline) = fetch_stats(config, args.show_all_sports)?;

  // Read battery status (non-fatal if unavailable)
  let battery = match ina219::Ina219::new().and_then(|mut ina| ina.read_status()) {
    Ok(status) => {
      log::info!("Battery: {status}");
      Some(status)
    },
    Err(e) => {
      log::info!("Battery monitor unavailable: {e}");
      None
    },
  };

  // Render
  let polyline_thickness = args.polyline_thickness.unwrap_or(config.display.polyline_thickness);
  let display_config = display::config::DisplayConfig { polyline_thickness,
                                                        ..config.display.clone() };
  let scale = display::renderer::Scale::new(args.scale);
  let img = display::renderer::render_dashboard(&stats,
                                                battery.as_ref(),
                                                &display_config,
                                                avatar.as_deref(),
                                                is_offline,
                                                scale);

  // Save PNG if requested
  if let Some(ref path) = args.save_png {
    img.save(path)
       .map_err(|e| DashError::Display(display::errors::DisplayError::Render(e.to_string())))?;
    log::info!("Dashboard saved to {path}");
  }

  // Try to push to e-paper display.
  // Always render at 2x and downsample — the area averaging darkens anti-aliased
  // font edges so they survive the 6-color quantization as black instead of
  // vanishing to white.
  match display::epd7in3e::Epd7in3e::new() {
    Ok(mut epd) => {
      let ss_scale = display::renderer::Scale::new(2);
      let epd_img = display::renderer::render_dashboard(&stats,
                                                        battery.as_ref(),
                                                        &display_config,
                                                        avatar.as_deref(),
                                                        is_offline,
                                                        ss_scale);
      let epd_img =
        if display_config.flip { image::imageops::rotate180(&epd_img) } else { epd_img };
      let buf = display::palette::quantize_supersampled_to_epd_buffer(&epd_img, 800, 480);
      epd.display_image(&buf)?;
      epd.sleep()?;
      log::info!("E-paper display updated");
    },
    Err(e) => {
      log::info!("E-paper display not available: {e}");
      if args.save_png.is_none() {
        // Auto-save PNG fallback when no display and no explicit save path
        let fallback_path = "dashboard_preview.png";
        img.save(fallback_path)
           .map_err(|e| DashError::Display(display::errors::DisplayError::Render(e.to_string())))?;
        log::info!("Dashboard saved to {fallback_path} (no display available)");
      }
    },
  }

  stats.print_summary();
  Ok(())
}

/// Fetch Strava data and compute dashboard stats. Also fetches/caches the
/// avatar. Falls back to cached (possibly stale) data when the network is
/// unavailable.
fn fetch_stats(config: &mut Config,
               show_all_sports: bool)
               -> Result<(common::DashboardStats, Option<Vec<u8>>, bool)> {
  match fetch_stats_online(config, show_all_sports) {
    Ok((stats, avatar)) => Ok((stats, avatar, false)),
    Err(DashError::Strava(strava::errors::StravaError::NetworkUnavailable(ref msg))) => {
      log::warn!("Network unavailable ({msg}), falling back to cached data");
      let (stats, avatar) = fetch_stats_from_cache(&config.display, show_all_sports)?;
      Ok((stats, avatar, true))
    },
    Err(e) => Err(e),
  }
}

/// Online path: authenticate, fetch from Strava API, cache results.
fn fetch_stats_online(config: &mut Config,
                      show_all_sports: bool)
                      -> Result<(common::DashboardStats, Option<Vec<u8>>)> {
  let mut client = strava::client::Client::new(config.strava.clone());
  client.get_token()?;
  log::info!("Fetching Strava stats");

  log::debug!("Getting athlete");
  let athlete = client.get_athlete()?;
  log::debug!("Athlete: {} (id: {})", athlete.full_name(), athlete.id);

  // Fetch avatar (non-fatal if unavailable)
  let avatar = load_or_fetch_avatar(&client, athlete.profile.as_deref());

  log::debug!("Getting athlete stats");
  let stats = client.get_athlete_stats(athlete.id)?;

  let year_start = NaiveDate::from_ymd_opt(Utc::now().year(), 1, 1).unwrap()
                                                                   .and_hms_opt(0, 0, 0)
                                                                   .unwrap()
                                                                   .and_utc()
                                                                   .timestamp();

  log::debug!("Getting activities since {year_start}");
  let activities = client.get_activities(year_start)?;
  log::debug!("Fetched {} activities", activities.len());

  // Persist updated refresh token if it changed
  if client.token_refreshed() {
    let new_token = client.refresh_token().to_string();
    config.strava.set_refresh_token(new_token);
    if let Err(e) = config.save() {
      log::warn!("Failed to save updated refresh token: {e}");
    }
  }

  let display_cfg = &config.display;
  let dashboard = strava::stats::compute(&stats,
                                         &activities,
                                         athlete.firstname.as_deref().unwrap_or("Athlete"),
                                         show_all_sports,
                                         |sport| display_cfg.longest_by_for(sport));

  Ok((dashboard, avatar))
}

/// Offline fallback: load stale cached data and build dashboard from whatever
/// is available. Scans per-athlete subdirectories and picks the most recently
/// used one.
fn fetch_stats_from_cache(display_cfg: &display::config::DisplayConfig,
                          show_all_sports: bool)
                          -> Result<(common::DashboardStats, Option<Vec<u8>>)> {
  let cache = strava::cache::Cache::new();

  let athlete_cache =
    cache.most_recent_athlete_cache()
         .ok_or_else(|| DashError::Config("No cached athlete data found".to_string()))?;

  let athlete: Option<strava::types::DetailedAthlete> = athlete_cache.load_stale("athlete");
  let firstname = athlete.as_ref().and_then(|a| a.firstname.as_deref()).unwrap_or("Athlete");

  let stats: strava::types::AthleteStats = athlete_cache.load_stale("stats").unwrap_or_default();

  let activities: Vec<strava::types::SummaryActivity> =
    athlete_cache.load_stale("activities").unwrap_or_default();

  let avatar = std::fs::read(athlete_cache.dir().join("avatar.img")).ok();

  log::info!("Offline fallback: athlete={}, activities={}", firstname, activities.len(),);

  let dashboard =
    strava::stats::compute(&stats, &activities, firstname, show_all_sports, |sport| {
      display_cfg.longest_by_for(sport)
    });

  Ok((dashboard, avatar))
}

/// Load avatar from cache or fetch from Strava CDN.
/// Uses the per-athlete cache directory (available after `get_athlete`).
fn load_or_fetch_avatar(client: &strava::client::Client,
                        profile_url: Option<&str>)
                        -> Option<Vec<u8>> {
  let cache_path = client.cache_dir().join("avatar.img");

  // Use cached file if it exists
  if cache_path.exists() {
    match std::fs::read(&cache_path) {
      Ok(bytes) if !bytes.is_empty() => {
        log::debug!("Avatar loaded from cache");
        return Some(bytes);
      },
      _ => {},
    }
  }

  // Download from URL
  let url = profile_url?;
  log::info!("Downloading avatar from {url}");
  match client.download_bytes(url) {
    Ok(bytes) => {
      if let Some(parent) = cache_path.parent() {
        let _ = std::fs::create_dir_all(parent);
      }
      let _ = std::fs::write(&cache_path, &bytes);
      Some(bytes)
    },
    Err(e) => {
      log::warn!("Failed to download avatar: {e}");
      None
    },
  }
}
