use clap::{builder::styling, CommandFactory, FromArgMatches};

mod errors;
use errors::Result;

mod args;
use args::Args;

use chrono::{Datelike, Local, NaiveDate, Timelike, Utc};
use std::thread;
use std::time::Duration;

use crate::errors::DashError;

const STYLES: styling::Styles = styling::Styles::styled()
    .header(styling::AnsiColor::Green.on_default().bold())
    .usage(styling::AnsiColor::Green.on_default().bold())
    .literal(styling::AnsiColor::Blue.on_default().bold())
    .placeholder(styling::AnsiColor::Cyan.on_default());

static PROGRAM_NAME: &str = env!("CARGO_PKG_NAME");

fn main() {
    env_logger::init();

    let result = run();

    match result {
        Ok(_) => {}
        Err(e) => {
            eprintln!("{} - error: {:?}", PROGRAM_NAME, e);
            std::process::exit(1);
        }
    }
}

/// Run the OAuth authorization flow explicitly, then save the refresh token.
fn run_auth() -> Result<()> {
    let mut config = strava::config::Config::load_for_auth().map_err(errors::DashError::Config)?;

    let token_response =
        strava::oauth::run_auth_flow(&config).map_err(errors::DashError::Strava)?;

    config.set_refresh_token(token_response.refresh_token);
    config.save().map_err(errors::DashError::Config)?;

    eprintln!("Authorization successful! Refresh token saved to config.");
    Ok(())
}

fn run() -> Result<()> {
    // Arguments
    let mut matches = Args::command().styles(STYLES).term_width(80).get_matches();
    let args =
        Args::from_arg_matches_mut(&mut matches).map_err(|e| DashError::Argument(e.to_string()))?;

    if args.auth {
        return run_auth();
    }

    // Load config
    let mut config = strava::config::Config::load().map_err(errors::DashError::Config)?;
    log::info!("Config loaded successfully");

    let sleep_secs = config.display.sleep_interval_secs;

    loop {
        // Check if we're inside the quiet window
        if !args.once && is_quiet_time(&config.display) {
            let secs = seconds_until_quiet_end(&config.display);
            log::info!(
                "Quiet hours ({:02}:00–{:02}:00) — sleeping for {secs}s until wake",
                config.display.quiet_start_hour,
                config.display.quiet_end_hour,
            );
            thread::sleep(Duration::from_secs(secs));
            // After waking, fall through to run a cycle immediately
        }

        match try_cycle(&config, &args) {
            Ok(()) => {}
            Err(DashError::Strava(strava::errors::StravaError::Unauthorized)) => {
                log::warn!("Unauthorized — attempting OAuth re-authorization");
                eprintln!("\nReceived 401 Unauthorized. Starting OAuth authorization flow...");

                let token_response = strava::oauth::run_auth_flow(&config)?;
                config.set_refresh_token(token_response.refresh_token);
                config.save().map_err(errors::DashError::Config)?;

                // Retry once after re-auth
                if let Err(e) = try_cycle(&config, &args) {
                    eprintln!("Error after re-authorization: {e:?}");
                }
            }
            Err(e) => {
                eprintln!("Error during cycle: {e:?}");
            }
        }

        if args.once {
            break;
        }

        log::info!("Sleeping for {} seconds...", sleep_secs);
        thread::sleep(Duration::from_secs(sleep_secs));
    }

    Ok(())
}

/// Run one full cycle: fetch stats → render image → display (or save PNG).
fn try_cycle(config: &strava::config::Config, args: &Args) -> Result<()> {
    let stats = fetch_stats(config)?;

    // Read battery status (non-fatal if unavailable)
    let battery = match display::ina219::Ina219::new().and_then(|mut ina| ina.read_status()) {
        Ok(status) => {
            log::info!(
                "Battery: {}% ({:.2}V, {})",
                status.percentage,
                status.voltage,
                if status.is_charging {
                    "charging"
                } else {
                    "discharging"
                }
            );
            Some(status)
        }
        Err(e) => {
            log::debug!("Battery monitor unavailable: {e}");
            None
        }
    };

    // Render
    let display_config = display::renderer::DisplayConfig {
        run_goal_km: config.display.run_goal_km,
        ride_goal_km: config.display.ride_goal_km,
    };
    let img = display::renderer::render_dashboard(&stats, battery.as_ref(), &display_config);

    // Save PNG if requested
    if let Some(ref path) = args.save_png {
        img.save(path)
            .map_err(|e| DashError::Display(display::errors::DisplayError::Render(e.to_string())))?;
        log::info!("Dashboard saved to {path}");
    }

    // Try to push to e-paper display
    match display::epd7in3e::Epd7in3e::new() {
        Ok(mut epd) => {
            let buf = display::palette::quantize_to_epd_buffer(&img);
            epd.display_image(&buf)?;
            epd.sleep()?;
            log::info!("E-paper display updated");
        }
        Err(e) => {
            log::info!("E-paper display not available: {e}");
            if args.save_png.is_none() {
                // Auto-save PNG fallback when no display and no explicit save path
                let fallback_path = "dashboard_preview.png";
                img.save(fallback_path).map_err(|e| {
                    DashError::Display(display::errors::DisplayError::Render(e.to_string()))
                })?;
                log::info!("Dashboard saved to {fallback_path} (no display available)");
            }
        }
    }

    stats.print_summary();
    Ok(())
}

/// Fetch Strava data and compute dashboard stats.
fn fetch_stats(config: &strava::config::Config) -> Result<strava::stats::DashboardStats> {
    let mut client = strava::client::Client::new(config.clone());
    client.get_token()?;

    log::info!("Getting athlete");
    let athlete = client.get_athlete()?;
    log::info!("Athlete: {} (id: {})", athlete.full_name(), athlete.id);

    log::info!("Getting athlete stats");
    let stats = client.get_athlete_stats(athlete.id)?;

    let year_start = NaiveDate::from_ymd_opt(Utc::now().year(), 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp();

    log::info!("Getting activities since {year_start}");
    let activities = client.get_activities(year_start)?;
    log::info!("Fetched {} activities", activities.len());

    Ok(strava::stats::DashboardStats::compute(
        &stats,
        &activities,
        &athlete.firstname.as_deref().unwrap_or("Athlete"),
    ))
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
