use crate::config::Config;
use crate::errors::{DashError, Result};
use chrono::{Datelike, NaiveDate, Utc};

pub struct Fetched {
  pub stats:      common::DashboardStats,
  pub avatar:     Option<Vec<u8>>,
  pub is_offline: bool,
}

/// Fetch Strava data and compute dashboard stats.
/// Tries fresh cache first (no client needed), then falls back to online fetch,
/// then to stale cache if the network is unavailable.
pub fn fetch(config: &mut Config,
             client: &mut Option<strava::client::Client>,
             show_all_sports: bool)
             -> Result<Fetched> {
  if let Some(fetched) = try_fresh_cache(&config.display, show_all_sports) {
    return Ok(fetched);
  }

  match fetch_online(config, client, show_all_sports) {
    Ok((stats, avatar)) => Ok(Fetched { stats,
                                        avatar,
                                        is_offline: false }),
    Err(DashError::Strava(strava::errors::StravaError::NetworkUnavailable(ref msg))) => {
      log::warn!("Network unavailable ({msg}), falling back to cached data");
      let (stats, avatar) = fetch_from_stale_cache(&config.display, show_all_sports)?;
      Ok(Fetched { stats,
                   avatar,
                   is_offline: true })
    },
    Err(e) => Err(e),
  }
}

/// Check if all required data is available in fresh cache.
/// Returns None if any cache entry is stale or missing.
fn try_fresh_cache(display_cfg: &display::config::DisplayConfig,
                   show_all_sports: bool)
                   -> Option<Fetched> {
  let cache = strava::cache::Cache::new();
  let athlete_cache = cache.most_recent_athlete_cache()?;

  let athlete: strava::types::DetailedAthlete = athlete_cache.load("athlete")?;
  let stats: strava::types::AthleteStats = athlete_cache.load("stats")?;
  let activities: Vec<strava::types::SummaryActivity> = athlete_cache.load("activities")?;
  let avatar = std::fs::read(athlete_cache.dir().join("avatar.img")).ok();

  let firstname = athlete.firstname.as_deref().unwrap_or("Athlete");
  log::info!("All cache fresh: athlete={}, activities={}", firstname, activities.len());

  let dashboard =
    strava::stats::compute(&stats, &activities, firstname, show_all_sports, |sport| {
      display_cfg.longest_by_for(sport)
    });

  Some(Fetched { stats: dashboard,
                 avatar,
                 is_offline: false })
}

fn get_or_create_client<'a>(config: &Config,
                            client: &'a mut Option<strava::client::Client>)
                            -> &'a mut strava::client::Client {
  if client.is_none() {
    *client = Some(strava::client::Client::new(config.strava.clone()));
  }
  client.as_mut().unwrap()
}

fn fetch_online(config: &mut Config,
                client: &mut Option<strava::client::Client>,
                show_all_sports: bool)
                -> Result<(common::DashboardStats, Option<Vec<u8>>)> {
  let c = get_or_create_client(config, client);
  c.get_token()?;
  log::info!("Fetching Strava stats");

  log::debug!("Getting athlete");
  let athlete = c.get_athlete()?;
  log::debug!("Athlete: {} (id: {})", athlete.full_name(), athlete.id);

  let avatar = load_or_fetch_avatar(c, athlete.profile.as_deref());

  log::debug!("Getting athlete stats");
  let stats = c.get_athlete_stats(athlete.id)?;

  let year_start = NaiveDate::from_ymd_opt(Utc::now().year(), 1, 1).unwrap()
                                                                   .and_hms_opt(0, 0, 0)
                                                                   .unwrap()
                                                                   .and_utc()
                                                                   .timestamp();

  log::debug!("Getting activities since {year_start}");
  let activities = c.get_activities(year_start)?;
  log::debug!("Fetched {} activities", activities.len());

  if c.token_refreshed() {
    let new_token = c.refresh_token().to_string();
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

fn fetch_from_stale_cache(display_cfg: &display::config::DisplayConfig,
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

  log::info!("Offline fallback: athlete={}, activities={}", firstname, activities.len());

  let dashboard =
    strava::stats::compute(&stats, &activities, firstname, show_all_sports, |sport| {
      display_cfg.longest_by_for(sport)
    });

  Ok((dashboard, avatar))
}

fn load_or_fetch_avatar(client: &strava::client::Client,
                        profile_url: Option<&str>)
                        -> Option<Vec<u8>> {
  let cache_path = client.cache_dir().join("avatar.img");

  if cache_path.exists()
     && let Ok(bytes) = std::fs::read(&cache_path)
     && !bytes.is_empty()
  {
    log::debug!("Avatar loaded from cache");
    return Some(bytes);
  }

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
