mod errors;
use errors::Result;

use chrono::{Datelike, NaiveDate, Utc};

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

fn run() -> Result<()> {
    // 1. Load config
    let config = strava::config::Config::load().map_err(errors::DashError::Config)?;
    log::info!("Config loaded successfully");

    // 2. Create client
    let mut client = strava::client::Client::new(config);

    // 3. Get token
    client
        .get_token()
        .map_err(|e| errors::DashError::Strava(e.to_string()))?;

    // 4. Get athlete (cached)
    log::info!("Getting athlete");
    let athlete = client
        .get_athlete()
        .map_err(|e| errors::DashError::Strava(e.to_string()))?;
    log::info!("Athlete: {} (id: {})", athlete.full_name(), athlete.id);

    // 5. Get athlete stats (cached)
    log::info!("Getting athlete stats");
    let stats = client
        .get_athlete_stats(athlete.id)
        .map_err(|e| errors::DashError::Strava(e.to_string()))?;

    // 6. Get activities for this year (cached)
    let year_start = NaiveDate::from_ymd_opt(Utc::now().year(), 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp();

    log::info!("Getting activities since {year_start}");
    let activities = client
        .get_activities(year_start)
        .map_err(|e| errors::DashError::Strava(e.to_string()))?;
    log::info!("Fetched {} activities", activities.len());

    // 7. Compute and display dashboard stats
    let dashboard = strava::stats::DashboardStats::compute(&stats, &activities);
    dashboard.print_summary();

    Ok(())
}
