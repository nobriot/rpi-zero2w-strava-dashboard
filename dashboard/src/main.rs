use clap::builder::styling;
use clap::{CommandFactory, FromArgMatches};

mod errors;
use errors::Result;

mod args;
mod power;
use crate::errors::DashError;
use args::Args;
use chrono::{Datelike, Local, NaiveDate, Timelike, Utc};
use std::path::PathBuf;

const STYLES: styling::Styles =
  styling::Styles::styled().header(styling::AnsiColor::Green.on_default().bold())
                           .usage(styling::AnsiColor::Green.on_default().bold())
                           .literal(styling::AnsiColor::Blue.on_default().bold())
                           .placeholder(styling::AnsiColor::Cyan.on_default());

static PROGRAM_NAME: &str = env!("CARGO_PKG_NAME");

fn main() {
  env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

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
                     Some(path) => strava::config::Config::load_from_for_auth(path),
                     None => strava::config::Config::load_for_auth(),
                   }.map_err(errors::DashError::Config)?;

  if !config.has_credentials() {
    eprintln!("Strava Dashboard — First-time setup");
    eprintln!("====================================");
    eprintln!();
    eprintln!("Create a Strava API application at: https://www.strava.com/settings/api");
    eprintln!("Set the \"Authorization Callback Domain\" to: localhost");
    eprintln!();

    let client_id = prompt("Client ID: ")?;
    let client_secret = prompt("Client Secret: ")?;

    config.set_client_id(client_id);
    config.set_client_secret(client_secret);
  }

  let token_response = strava::oauth::run_auth_flow(&config).map_err(errors::DashError::Strava)?;

  config.set_refresh_token(token_response.refresh_token);
  config.save().map_err(errors::DashError::Config)?;

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
  // Arguments
  let mut matches = Args::command().styles(STYLES).term_width(80).get_matches();
  let args =
    Args::from_arg_matches_mut(&mut matches).map_err(|e| DashError::Argument(e.to_string()))?;

  if args.auth {
    return run_auth(args.config.as_ref());
  }

  if args.clear_cache {
    strava::cache::Cache::new().clear().map_err(errors::DashError::Config)?;
    eprintln!("Cache cleared.");
  }

  // Load config
  let mut config = match args.config.as_ref() {
                     Some(path) => strava::config::Config::load_from(path),
                     None => strava::config::Config::load(),
                   }.map_err(errors::DashError::Config)?;
  log::info!("Config loaded successfully");

  loop {
    match try_cycle(&config, &args) {
      Ok(()) => {},
      Err(DashError::Strava(strava::errors::StravaError::Unauthorized)) => {
        // Token refresh already attempted by the client.
        // If we still get Unauthorized, the refresh token is invalid — need full OAuth.
        log::warn!("Unauthorized after auto-refresh — attempting full OAuth re-authorization");
        eprintln!("\nRefresh token invalid. Starting OAuth authorization flow...");

        let token_response = strava::oauth::run_auth_flow(&config)?;
        config.set_refresh_token(token_response.refresh_token);
        config.save().map_err(errors::DashError::Config)?;

        if let Err(e) = try_cycle(&config, &args) {
          eprintln!("Error after re-authorization: {e:?}");
        }
      },
      Err(DashError::Strava(strava::errors::StravaError::NetworkUnavailable(ref msg))) => {
        // fetch_stats already falls back to cached data, so this is unlikely
        // to be reached. Log and retry next cycle.
        log::warn!("Network unavailable: {msg}");
        eprintln!("Network unavailable — will retry next cycle");
      },
      Err(e) => {
        eprintln!("Error during cycle: {e:?}");
      },
    }

    if args.once {
      break;
    }

    // Fresh battery read for power decision
    let battery = power::read_battery();
    let on_power = battery.as_ref().is_none_or(|b| b.is_charging);
    let battery_pct = battery.as_ref().map(|b| b.percentage);

    // On external power (or no battery sensor): short interval, ignore quiet
    // hours, no shutdown. This also covers dev machines without an INA219.
    if on_power {
      let secs = config.power.charging_interval_secs;
      log::info!("On power — sleeping {secs}s");
      std::thread::sleep(std::time::Duration::from_secs(secs));
      continue;
    }

    // Battery mode: respect quiet hours
    let sleep_duration = if is_quiet_time(&config.display) {
      let secs = seconds_until_quiet_end(&config.display);
      log::info!("Quiet hours ({:02}:00–{:02}:00) — sleeping {secs}s until wake",
                 config.display.quiet_start_hour,
                 config.display.quiet_end_hour,);
      secs
    } else {
      let secs = config.display.sleep_interval_secs;
      log::info!("Battery mode — sleeping {secs}s");
      secs
    };

    // Linger: stay awake briefly so a user can SSH in (capped to sleep duration)
    let linger = config.power.linger_secs.min(sleep_duration);
    if linger > 0 {
      log::info!("Lingering {linger}s for SSH access…");
      std::thread::sleep(std::time::Duration::from_secs(linger));
    }

    let remaining = sleep_duration.saturating_sub(linger);

    // Decide whether to rtcwake-shutdown
    let ssh_active = power::has_ssh_sessions();
    let ssh_inhibits = config.power.ssh_inhibit_below_percent > 0
                       && ssh_active
                       && battery_pct.is_none_or(|p| p > config.power.ssh_inhibit_below_percent);

    log::info!("Power: battery={}, ssh={ssh_active}, ssh_inhibits={ssh_inhibits}, \
                shutdown_after_cycle={}",
               battery_pct.map_or("N/A".to_string(), |p| format!("{p}%")),
               config.power.shutdown_after_cycle,);

    if config.power.shutdown_after_cycle
       && !ssh_inhibits
       && remaining > 0
       && power::try_rtcwake_shutdown(remaining)
    {
      break;
    }

    if remaining > 0 {
      std::thread::sleep(std::time::Duration::from_secs(remaining));
    }
  }

  Ok(())
}

/// Run one full cycle: fetch stats → render image → display (or save PNG).
fn try_cycle(config: &strava::config::Config, args: &Args) -> Result<()> {
  let (stats, avatar, is_offline) = fetch_stats(config, args.show_all_sports)?;

  // Read battery status (non-fatal if unavailable)
  let battery = match display::ina219::Ina219::new().and_then(|mut ina| ina.read_status()) {
    Ok(status) => {
      log::info!("Battery: {}% ({:.2}V, {})",
                 status.percentage,
                 status.voltage,
                 if status.is_charging { "charging" } else { "discharging" });
      Some(status)
    },
    Err(e) => {
      log::info!("Battery monitor unavailable: {e}");
      None
    },
  };

  // Render
  let polyline_thickness = args.polyline_thickness.unwrap_or(config.display.polyline_thickness);
  let display_config =
    display::renderer::DisplayConfig { goals: config.display.goals.clone(),
                                       polyline_thickness,
                                       show_totals: config.display.show_totals,
                                       show_longest_fastest: config.display.show_longest_fastest };
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
fn fetch_stats(config: &strava::config::Config,
               show_all_sports: bool)
               -> Result<(strava::stats::DashboardStats, Option<Vec<u8>>, bool)> {
  match fetch_stats_online(config, show_all_sports) {
    Ok((stats, avatar)) => Ok((stats, avatar, false)),
    Err(DashError::Strava(strava::errors::StravaError::NetworkUnavailable(ref msg))) => {
      log::warn!("Network unavailable ({msg}), falling back to cached data");
      let (stats, avatar) = fetch_stats_from_cache(show_all_sports)?;
      Ok((stats, avatar, true))
    },
    Err(e) => Err(e),
  }
}

/// Online path: authenticate, fetch from Strava API, cache results.
fn fetch_stats_online(config: &strava::config::Config,
                      show_all_sports: bool)
                      -> Result<(strava::stats::DashboardStats, Option<Vec<u8>>)> {
  let mut client = strava::client::Client::new(config.clone());
  client.get_token()?;

  log::info!("Getting athlete");
  let athlete = client.get_athlete()?;
  log::info!("Athlete: {} (id: {})", athlete.full_name(), athlete.id);

  // Fetch avatar (non-fatal if unavailable)
  let avatar = load_or_fetch_avatar(&client, athlete.profile.as_deref());

  log::info!("Getting athlete stats");
  let stats = client.get_athlete_stats(athlete.id)?;

  let year_start = NaiveDate::from_ymd_opt(Utc::now().year(), 1, 1).unwrap()
                                                                   .and_hms_opt(0, 0, 0)
                                                                   .unwrap()
                                                                   .and_utc()
                                                                   .timestamp();

  log::info!("Getting activities since {year_start}");
  let activities = client.get_activities(year_start)?;
  log::info!("Fetched {} activities", activities.len());

  let dashboard =
    strava::stats::DashboardStats::compute(&stats,
                                           &activities,
                                           athlete.firstname.as_deref().unwrap_or("Athlete"),
                                           show_all_sports);

  Ok((dashboard, avatar))
}

/// Offline fallback: load stale cached data and build dashboard from whatever
/// is available. Scans per-athlete subdirectories and picks the most recently
/// used one.
fn fetch_stats_from_cache(show_all_sports: bool)
                          -> Result<(strava::stats::DashboardStats, Option<Vec<u8>>)> {
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
    strava::stats::DashboardStats::compute(&stats, &activities, firstname, show_all_sports);

  Ok((dashboard, avatar))
}

/// Check whether the current local time falls inside the quiet window.
fn is_quiet_time(display: &strava::config::DisplayConfig) -> bool {
  let hour = Local::now().hour();
  let start = display.quiet_start_hour;
  let end = display.quiet_end_hour;

  if start <= end {
    // e.g. quiet 02:00–06:00 (no midnight wrap)
    hour >= start && hour < end
  } else {
    // e.g. quiet 20:00–08:00 (wraps midnight)
    hour >= start || hour < end
  }
}

/// Compute seconds from now until the quiet window ends.
fn seconds_until_quiet_end(display: &strava::config::DisplayConfig) -> u64 {
  let now = Local::now();
  let hour = now.hour();
  let end = display.quiet_end_hour;

  // Hours remaining until the end hour
  let hours_left = if hour < end {
    end - hour
  } else {
    // Past midnight wrap: remaining hours today + hours into tomorrow
    (24 - hour) + end
  };

  let minutes_left = 60 - now.minute();
  // Subtract one hour because the minutes already cover part of it,
  // but ensure we don't underflow.
  let total_secs = if hours_left > 0 {
    ((hours_left - 1) as u64 * 3600) + (minutes_left as u64 * 60)
  } else {
    minutes_left as u64 * 60
  };

  // At least 60 seconds to avoid a busy-loop from rounding
  total_secs.max(60)
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
        log::info!("Avatar loaded from cache");
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
