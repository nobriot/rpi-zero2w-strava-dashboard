use clap::builder::styling;
use clap::{CommandFactory, FromArgMatches};

mod args;
mod config;
mod errors;
mod firmware;
mod power;

use args::Args;
use chrono::{Datelike, Duration, Local, NaiveDate, Timelike, Utc};
use config::Config;
use errors::{DashError, Result};
use std::io::IsTerminal;
use std::path::PathBuf;

const STYLES: styling::Styles =
  styling::Styles::styled().header(styling::AnsiColor::Green.on_default().bold())
                           .usage(styling::AnsiColor::Green.on_default().bold())
                           .literal(styling::AnsiColor::Blue.on_default().bold())
                           .placeholder(styling::AnsiColor::Cyan.on_default());

static PROGRAM_NAME: &str = env!("CARGO_PKG_NAME");

const CYCLE_SECONDS_ON_POWER: u64 = 60;

fn main() {
  let mut log_builder =
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"));
  if std::io::stderr().is_terminal() {
    // Compact timestamp + colored level for interactive use
    log_builder.format(|buf, record| {
                 use std::io::Write;
                 let now = chrono::Local::now();
                 let level = record.level();
                 let style = buf.default_level_style(level);
                 writeln!(buf,
                          "{} [{style}{}{style:#} {}] {}",
                          now.format("%Y/%m/%d %H:%M"),
                          level,
                          record.module_path().unwrap_or(""),
                          record.args())
               });
  } else {
    // No timestamp under systemd (journalctl provides its own)
    log_builder.format(|buf, record| {
                 use std::io::Write;
                 writeln!(buf,
                          "[{} {}] {}",
                          record.level(),
                          record.module_path().unwrap_or(""),
                          record.args())
               });
  }
  log_builder.init();

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
  // Arguments
  let mut matches = Args::command().styles(STYLES).term_width(80).get_matches();
  let args =
    Args::from_arg_matches_mut(&mut matches).map_err(|e| DashError::Argument(e.to_string()))?;

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

  let mut peripherals = power::Peripherals::new();

  loop {
    // Re-enable WiFi if it was disabled during the previous sleep
    peripherals.enable_wifi();

    match try_cycle(&mut config, &args) {
      Ok(()) => {},
      Err(DashError::Strava(strava::errors::StravaError::Unauthorized)) => {
        // Token refresh already attempted by the client.
        // If we still get Unauthorized, the refresh token is invalid — need full OAuth.
        log::warn!("Unauthorized after auto-refresh — attempting full OAuth re-authorization");
        eprintln!("\nRefresh token invalid. Starting OAuth authorization flow...");

        let token_response = strava::oauth::run_auth_flow(&config.strava)?;
        config.strava.set_refresh_token(token_response.refresh_token);
        config.save().map_err(DashError::Config)?;

        if let Err(e) = try_cycle(&mut config, &args) {
          eprintln!("Error after re-authorization: {e:?}");
        }
      },
      Err(DashError::Strava(strava::errors::StravaError::NetworkUnavailable(ref msg))) => {
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
      peripherals.set_normal();
      let secs = CYCLE_SECONDS_ON_POWER;
      log::info!("On power -- sleeping {secs}s before next cycle");
      std::thread::sleep(std::time::Duration::from_secs(secs));
      continue;
    } else {
      peripherals.set_low_power();
    }

    // Battery mode: respect quiet hours
    let sleep_duration = if is_quiet_time(&config.power) {
      let secs = seconds_until_quiet_end(&config.power);
      log::info!("Quiet hours ({:02}:00-{:02}:00) -- sleeping {secs}s until wake",
                 config.power.quiet_hours.start,
                 config.power.quiet_hours.end,);
      secs
    } else {
      let secs = seconds_until_next_slot(&config.power, config.power.sleep_interval_secs);
      log::info!("Battery mode -- sleeping {secs}s (next grid slot)");
      secs
    };

    // Linger: stay awake so a user can SSH in, but cut short once they log off
    let linger = config.power.linger_secs.min(sleep_duration);
    if linger > 0 {
      log::info!("Lingering {linger}s for SSH access...");
      let mut waited = 0u64;
      while waited < linger {
        let chunk = 60.min(linger - waited);
        std::thread::sleep(std::time::Duration::from_secs(chunk));
        waited += chunk;
        if waited < linger && !power::has_ssh_sessions() {
          log::info!("No SSH sessions -- ending linger early");
          break;
        }
      }
    }

    let remaining = sleep_duration.saturating_sub(linger);

    // Check SSH state for shutdown inhibition
    let ssh_active = power::has_ssh_sessions();
    let ssh_inhibits = config.power.ssh_inhibit_below_percent > 0
                       && ssh_active
                       && battery_pct.is_none_or(|p| p > config.power.ssh_inhibit_below_percent);

    log::info!("Power: battery={}, ssh={ssh_active}, ssh_inhibits={ssh_inhibits}, \
                shutdown_after_cycle={}",
               battery_pct.map_or("N/A".to_string(), |p| format!("{p}%")),
               config.power.shutdown_after_cycle,);

    if ssh_inhibits {
      // SSH active -- poll every 60s until sessions end, then sleep/shutdown
      log::info!("SSH inhibiting shutdown -- polling every 60s");
      let mut waited = 0u64;
      while waited < remaining {
        let chunk = 60.min(remaining - waited);
        std::thread::sleep(std::time::Duration::from_secs(chunk));
        waited += chunk;
        if !power::has_ssh_sessions() {
          log::info!("SSH sessions ended -- proceeding to sleep");
          peripherals.disable_wifi();
          let left = remaining.saturating_sub(waited);
          if left > 0 && config.power.shutdown_after_cycle && power::try_rtcwake_shutdown(left) {
            return Ok(());
          }
          if left > 0 {
            std::thread::sleep(std::time::Duration::from_secs(left));
          }
          break;
        }
      }
    } else {
      // No SSH -- disable WiFi and sleep/shutdown immediately
      peripherals.disable_wifi();

      if config.power.shutdown_after_cycle
         && remaining > 0
         && power::try_rtcwake_shutdown(remaining)
      {
        break;
      }

      if remaining > 0 {
        std::thread::sleep(std::time::Duration::from_secs(remaining));
      }
    }
  }

  Ok(())
}

/// Run one full cycle: fetch stats -> render image -> display (or save PNG).
fn try_cycle(config: &mut Config, args: &Args) -> Result<()> {
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
      let (stats, avatar) = fetch_stats_from_cache(show_all_sports)?;
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

  let dashboard = strava::stats::compute(&stats,
                                         &activities,
                                         athlete.firstname.as_deref().unwrap_or("Athlete"),
                                         show_all_sports);

  Ok((dashboard, avatar))
}

/// Offline fallback: load stale cached data and build dashboard from whatever
/// is available. Scans per-athlete subdirectories and picks the most recently
/// used one.
fn fetch_stats_from_cache(show_all_sports: bool)
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

  let dashboard = strava::stats::compute(&stats, &activities, firstname, show_all_sports);

  Ok((dashboard, avatar))
}

/// Check whether the current local time falls inside the quiet window.
fn is_quiet_time(power: &config::PowerConfig) -> bool {
  let hour = Local::now().hour();
  let start = power.quiet_hours.start;
  let end = power.quiet_hours.end;

  if start <= end {
    // e.g. quiet 02:00-06:00 (no midnight wrap)
    hour >= start && hour < end
  } else {
    // e.g. quiet 20:00-08:00 (wraps midnight)
    hour >= start || hour < end
  }
}

/// Compute seconds from now until the quiet window ends.
fn seconds_until_quiet_end(power: &config::PowerConfig) -> u64 {
  let now = Local::now();
  let hour = now.hour();
  let end = power.quiet_hours.end;

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

/// Compute seconds until the next grid-aligned wake slot.
///
/// The grid is anchored at `quiet_hours.end` each day, with slots
/// spaced `interval_secs` apart.  For example, with quiet end=6 and
/// interval=1200 (20 min), the slots are 06:00, 06:20, 06:40, 07:00, ...
///
/// If the next slot would fall inside quiet hours, returns the seconds
/// until quiet end instead (i.e. the first slot of the next active window).
fn seconds_until_next_slot(power: &config::PowerConfig, interval_secs: u64) -> u64 {
  let now = Local::now();
  let today_anchor = now.date_naive()
                        .and_hms_opt(power.quiet_hours.end, 0, 0)
                        .expect("valid quiet_hours.end")
                        .and_local_timezone(Local)
                        .single()
                        .expect("unambiguous local time");

  // Use today's anchor if it's in the past, otherwise yesterday's.
  let anchor = if today_anchor <= now {
    today_anchor
  } else {
    today_anchor - Duration::days(1)
  };

  let elapsed = (now - anchor).num_seconds() as u64;
  let remainder = elapsed % interval_secs;
  let next_in = if remainder == 0 { interval_secs } else { interval_secs - remainder };

  // Check whether the target wake time would land in quiet hours.
  let wake_time = now + Duration::seconds(next_in as i64);
  let wake_hour = wake_time.hour();
  let in_quiet = {
    let start = power.quiet_hours.start;
    let end = power.quiet_hours.end;
    if start <= end {
      wake_hour >= start && wake_hour < end
    } else {
      wake_hour >= start || wake_hour < end
    }
  };

  if in_quiet { seconds_until_quiet_end(power) } else { next_in.max(60) }
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
