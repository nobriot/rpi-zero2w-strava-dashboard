use clap::{builder::styling, CommandFactory, FromArgMatches};

mod errors;
use errors::Result;

mod args;
use args::Args;

use chrono::{Datelike, NaiveDate, Utc};

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
        run_auth()?
    }

    // Load config
    let mut config = strava::config::Config::load().map_err(errors::DashError::Config)?;
    log::info!("Config loaded successfully");

    match try_run(&config) {
        Ok(()) => Ok(()),
        Err(errors::DashError::Strava(strava::errors::StravaError::Unauthorized)) => {
            log::warn!("Unauthorized — attempting OAuth re-authorization");
            eprintln!("\nReceived 401 Unauthorized. Starting OAuth authorization flow...");

            let token_response = strava::oauth::run_auth_flow(&config)?;
            config.set_refresh_token(token_response.refresh_token);
            config.save().map_err(errors::DashError::Config)?;

            try_run(&config)
        }
        Err(e) => Err(e),
    }
}

fn try_run(config: &strava::config::Config) -> Result<()> {
    let mut client = strava::client::Client::new(config.clone());
    // Get token
    client.get_token()?;

    // Get athlete (cached)
    log::info!("Getting athlete");
    let athlete = client.get_athlete()?;
    log::info!("Athlete: {} (id: {})", athlete.full_name(), athlete.id);

    // Get athlete stats (cached)
    log::info!("Getting athlete stats");
    let stats = client.get_athlete_stats(athlete.id)?;

    // Get activities for this year (cached)
    let year_start = NaiveDate::from_ymd_opt(Utc::now().year(), 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp();

    log::info!("Getting activities since {year_start}");
    let activities = client.get_activities(year_start)?;
    log::info!("Fetched {} activities", activities.len());

    // Compute and display dashboard stats
    let dashboard = strava::stats::DashboardStats::compute(&stats, &activities);
    dashboard.print_summary();

    Ok(())
}
