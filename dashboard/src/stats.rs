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
///
/// When `year_override` is `Some`, caches are bypassed entirely and activities
/// are fetched for that specific year only.
pub fn fetch(config: &mut Config,
             client: &mut Option<strava::client::Client>,
             show_all_sports: bool,
             year_override: Option<i32>)
             -> Result<Fetched> {
  let client_id = config.strava.client_id().to_string();
  if year_override.is_none()
     && let Some(fetched) = try_fresh_cache(&client_id, &config.display, show_all_sports)
  {
    return Ok(fetched);
  }

  match fetch_online(config, client, show_all_sports, year_override) {
    Ok((stats, avatar)) => Ok(Fetched { stats,
                                        avatar,
                                        is_offline: false }),
    Err(DashError::Strava(strava::errors::StravaError::NetworkUnavailable(ref msg))) => {
      if year_override.is_some() {
        return Err(DashError::Strava(strava::errors::StravaError::NetworkUnavailable(msg.clone())));
      }
      log::warn!("Network unavailable ({msg}), falling back to cached data");
      let (stats, avatar) = fetch_from_stale_cache(&client_id, &config.display, show_all_sports)?;
      Ok(Fetched { stats,
                   avatar,
                   is_offline: true })
    },
    Err(e) => Err(e),
  }
}

/// Look up the athlete ID for a given client_id from the cached mapping.
fn cached_athlete_id(client_id: &str) -> Option<u64> {
  let cache = strava::cache::Cache::new();
  let path = cache.dir().join(format!("athlete_for_{client_id}"));
  let content = std::fs::read_to_string(path).ok()?;
  content.trim().parse().ok()
}

/// Save the client_id -> athlete_id mapping to disk.
fn save_athlete_id(client_id: &str, athlete_id: u64) {
  let cache = strava::cache::Cache::new();
  let path = cache.dir().join(format!("athlete_for_{client_id}"));
  if let Some(parent) = path.parent() {
    let _ = std::fs::create_dir_all(parent);
  }
  let _ = std::fs::write(path, athlete_id.to_string());
}

/// Check if all required data is available in fresh cache.
/// Uses the client_id -> athlete_id mapping to find the right cache directory.
fn try_fresh_cache(client_id: &str,
                   display_cfg: &display::config::DisplayConfig,
                   show_all_sports: bool)
                   -> Option<Fetched> {
  let athlete_id = cached_athlete_id(client_id)?;
  let cache = strava::cache::Cache::new().for_athlete(athlete_id);

  let athlete: strava::types::DetailedAthlete = cache.load("athlete")?;
  let stats: strava::types::AthleteStats = cache.load("stats")?;
  let activities: Vec<strava::types::SummaryActivity> = cache.load("activities")?;
  let avatar = std::fs::read(cache.dir().join("avatar.img")).ok();

  let firstname = athlete.firstname.as_deref().unwrap_or("Athlete");
  log::info!("All cache fresh: athlete={}, activities={}", firstname, activities.len());

  let year = Utc::now().year();
  let dashboard =
    strava::stats::compute(&stats, &activities, firstname, show_all_sports, year, |sport| {
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
                show_all_sports: bool,
                year_override: Option<i32>)
                -> Result<(common::DashboardStats, Option<Vec<u8>>)> {
  let c = get_or_create_client(config, client);
  c.get_token()?;
  log::info!("Fetching Strava stats");

  log::debug!("Getting athlete");
  let athlete = c.get_athlete()?;
  log::debug!("Athlete: {} (id: {})", athlete.full_name(), athlete.id);

  save_athlete_id(config.strava.client_id(), athlete.id);

  let avatar = load_or_fetch_avatar(c, athlete.profile.as_deref());

  let year = year_override.unwrap_or_else(|| Utc::now().year());
  let year_start = jan_first_utc(year);

  let activities = if let Some(target_year) = year_override {
    let year_end = jan_first_utc(target_year + 1);
    log::debug!("Getting activities for year {target_year} ({year_start}..{year_end}), no cache");
    c.get_activities_range(year_start, Some(year_end), false)?
  } else {
    log::debug!("Getting activities since {year_start}");
    c.get_activities(year_start)?
  };
  log::debug!("Fetched {} activities", activities.len());

  let stats = if year_override.is_some() {
    strava::stats::synthesize_stats_from_activities(&activities)
  } else {
    log::debug!("Getting athlete stats");
    c.get_athlete_stats(athlete.id)?
  };

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
                                         year,
                                         |sport| display_cfg.longest_by_for(sport));

  Ok((dashboard, avatar))
}

fn jan_first_utc(year: i32) -> i64 {
  NaiveDate::from_ymd_opt(year, 1, 1).unwrap()
                                     .and_hms_opt(0, 0, 0)
                                     .unwrap()
                                     .and_utc()
                                     .timestamp()
}

/// Offline fallback using the client_id -> athlete_id mapping, or
/// most_recent_athlete_cache as a last resort.
fn fetch_from_stale_cache(client_id: &str,
                          display_cfg: &display::config::DisplayConfig,
                          show_all_sports: bool)
                          -> Result<(common::DashboardStats, Option<Vec<u8>>)> {
  let cache = strava::cache::Cache::new();

  let athlete_cache = if let Some(athlete_id) = cached_athlete_id(client_id) {
    cache.for_athlete(athlete_id)
  } else {
    cache.most_recent_athlete_cache()
         .ok_or_else(|| DashError::Config("No cached athlete data found".to_string()))?
  };

  let athlete: Option<strava::types::DetailedAthlete> = athlete_cache.load_stale("athlete");
  let firstname = athlete.as_ref().and_then(|a| a.firstname.as_deref()).unwrap_or("Athlete");

  let stats: strava::types::AthleteStats = athlete_cache.load_stale("stats").unwrap_or_default();

  let activities: Vec<strava::types::SummaryActivity> =
    athlete_cache.load_stale("activities").unwrap_or_default();

  let avatar = std::fs::read(athlete_cache.dir().join("avatar.img")).ok();

  log::info!("Offline fallback: athlete={}, activities={}", firstname, activities.len());

  let year = Utc::now().year();
  let dashboard =
    strava::stats::compute(&stats, &activities, firstname, show_all_sports, year, |sport| {
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
    log::debug!("Avatar loaded from cache: {} ({} bytes)", cache_path.display(), bytes.len());
    return Some(bytes);
  }

  let url = profile_url?;
  log::info!("Downloading avatar from {url}");
  match client.download_bytes(url) {
    Ok(bytes) => {
      log::debug!("Avatar downloaded: {} bytes", bytes.len());
      if let Some(parent) = cache_path.parent()
         && let Err(e) = std::fs::create_dir_all(parent)
      {
        log::warn!("Failed to create avatar cache directory {}: {e}", parent.display());
      }
      match std::fs::write(&cache_path, &bytes) {
        Ok(()) => log::debug!("Avatar cached at {}", cache_path.display()),
        Err(e) => log::warn!("Failed to write avatar cache {}: {e}", cache_path.display()),
      }
      Some(bytes)
    },
    Err(e) => {
      log::warn!("Failed to download avatar from {url}: {e}");
      None
    },
  }
}
