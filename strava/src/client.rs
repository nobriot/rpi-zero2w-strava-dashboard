use crate::cache::Cache;
use crate::config::StravaConfig;
use crate::errors::StravaError;
use crate::types::{AthleteStats, DetailedAthlete, SummaryActivity, TokenResponse};
use reqwest::blocking::Client as ReqwestClient;

pub struct Client {
  client:          ReqwestClient,
  config:          StravaConfig,
  cache:           Cache,
  access_token:    Option<String>,
  token_refreshed: bool,
}

impl Client {
  /// Query activities at most every 3 hours
  const ACTIVITIES_CACHE_TIME: u64 = 3600 * 3;
  /// Keep athele cache for 1 week before quering again
  const ATHELE_CACHE_TIME: u64 = 3600 * 24 * 7;
  /// Refresh generic stats at most once per day
  const STATS_CACHE_TIME: u64 = 3600 * 24;
  const STRAVA_API_BASE_URL: &str = "https://www.strava.com/api/v3/";
  const STRAVA_TOKEN_URL: &str = "https://www.strava.com/oauth/token";

  pub fn new(config: StravaConfig) -> Self {
    log::info!("Creating Strava Client");

    Self { client: ReqwestClient::new(),
           config,
           cache: Cache::new(),
           access_token: None,
           token_refreshed: false }
  }

  pub fn get_token(&mut self) -> Result<(), StravaError> {
    log::info!("Getting token");

    let response = self.client
                       .post(Self::STRAVA_TOKEN_URL)
                       .form(&[("client_id", self.config.client_id()),
                               ("client_secret", self.config.client_secret()),
                               ("grant_type", "refresh_token"),
                               ("refresh_token", self.config.refresh_token()),
                               ("scope", "read,activity:read_all")])
                       .send()
                       .map_err(Self::classify_reqwest_error)?;

    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
      return Err(StravaError::Unauthorized);
    }

    if !response.status().is_success() {
      return Err(StravaError::StravaApiResponseError(format!("Token refresh failed with status: {}",
                                                             response.status())));
    }

    let body = response.text().map_err(|e| StravaError::StravaApiResponseError(e.to_string()))?;

    let token_response: TokenResponse = serde_json::from_str(&body).map_err(|_| {
                                          StravaError::StravaApiResponseDeserializationError(body)
                                        })?;

    self.access_token = Some(token_response.access_token);

    // Update refresh token in memory if it changed — caller persists
    if token_response.refresh_token != self.config.refresh_token() {
      log::info!("Refresh token changed, updating in-memory config");
      self.config.set_refresh_token(token_response.refresh_token);
      self.token_refreshed = true;
    }

    log::info!("Access token obtained");
    Ok(())
  }

  /// Sends a GET request to the Strava API.
  /// On 401, automatically retries once after refreshing the access token.
  fn strava_api_get(&mut self,
                    api_endpoint: &str)
                    -> Result<reqwest::blocking::Response, StravaError> {
    if self.access_token.is_none() {
      self.get_token()?;
    }

    let response = self.do_api_get(api_endpoint)?;

    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
      log::info!("Got 401, attempting automatic token refresh");
      self.access_token = None;
      self.get_token()?;
      let retry = self.do_api_get(api_endpoint)?;

      if retry.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(StravaError::Unauthorized);
      }

      return Self::check_response(retry, api_endpoint);
    }

    Self::check_response(response, api_endpoint)
  }

  /// Low-level GET request (no retry logic).
  fn do_api_get(&self, api_endpoint: &str) -> Result<reqwest::blocking::Response, StravaError> {
    self.client
        .get(format!("{}{}", Self::STRAVA_API_BASE_URL, api_endpoint))
        .header("Authorization", format!("Bearer {}", self.access_token.as_ref().unwrap()))
        .header("Accept", "application/json")
        .send()
        .map_err(Self::classify_reqwest_error)
  }

  /// Check a non-401 response for success.
  fn check_response(response: reqwest::blocking::Response,
                    api_endpoint: &str)
                    -> Result<reqwest::blocking::Response, StravaError> {
    if !response.status().is_success() {
      let status = response.status();
      let text = response.text().unwrap_or(String::from(""));
      return Err(StravaError::StravaApiResponseError(format!("API request to '{}' failed with status: {} - body: {}",
                                                             api_endpoint, status, text)));
    }
    Ok(response)
  }

  /// GET /athlete — returns the authenticated athlete.
  /// Always fetches from the API so the correct athlete is returned for the
  /// current config (avoids cross-contamination when multiple accounts share
  /// the cache directory). Scopes the cache to a per-athlete subdirectory.
  pub fn get_athlete(&mut self) -> Result<DetailedAthlete, StravaError> {
    let response = self.strava_api_get("athlete")?;
    let body = response.text().map_err(|_| StravaError::StravaApiResponseMissingBody)?;
    log::debug!("JSON:\n{body}");

    let athlete: DetailedAthlete = serde_json::from_str(&body).map_err(|_| {
                                     StravaError::StravaApiResponseDeserializationError(body)
                                   })?;

    // Switch to per-athlete dir, then save athlete there
    self.cache = self.cache.for_athlete(athlete.id);
    self.cache.save("athlete", &athlete, Some(Self::ATHELE_CACHE_TIME));
    Ok(athlete)
  }

  /// The current cache directory (per-athlete after `get_athlete`).
  pub fn cache_dir(&self) -> &std::path::Path {
    self.cache.dir()
  }

  /// Download raw bytes from any URL (no authentication).
  /// Used for fetching avatar images, etc.
  pub fn download_bytes(&self, url: &str) -> Result<Vec<u8>, StravaError> {
    let response = self.client.get(url).send().map_err(Self::classify_reqwest_error)?;

    if !response.status().is_success() {
      return Err(StravaError::StravaApiResponseError(format!("Download failed with status: {}",
                                                             response.status())));
    }

    let bytes = response.bytes().map_err(|e| StravaError::StravaApiResponseError(e.to_string()))?;
    Ok(bytes.to_vec())
  }

  /// GET /athletes/{id}/stats — returns aggregate stats. Uses cache.
  pub fn get_athlete_stats(&mut self, athlete_id: u64) -> Result<AthleteStats, StravaError> {
    if let Some(cached) = self.cache.load::<AthleteStats>("stats") {
      return Ok(cached);
    }

    let response = self.strava_api_get(&format!("athletes/{athlete_id}/stats"))?;
    let body = response.text().map_err(|_| StravaError::StravaApiResponseMissingBody)?;
    log::debug!("JSON:\n{body}");

    let stats: Result<AthleteStats, serde_json::Error> = serde_json::from_str(&body);
    // dbg!(&stats);
    let stats = stats.map_err(|_| StravaError::StravaApiResponseDeserializationError(body))?;

    self.cache.save("stats", &stats, Some(Self::STATS_CACHE_TIME));
    Ok(stats)
  }

  /// GET /athlete/activities — paginated fetch of activities since `after`
  /// (unix timestamp). Uses cache.
  pub fn get_activities(&mut self, after: i64) -> Result<Vec<SummaryActivity>, StravaError> {
    if let Some(mut cached) = self.cache.load::<Vec<SummaryActivity>>("activities") {
      cached.sort_by(|a, b| b.start_date_local.as_deref().cmp(&a.start_date_local.as_deref()));
      return Ok(cached);
    }

    let mut all_activities: Vec<SummaryActivity> = Vec::new();
    let mut page = 1u32;
    const PER_PAGE: u32 = 200;
    const MAX_PAGES: u32 = 10;

    loop {
      if page > MAX_PAGES {
        log::warn!("Reached max pages ({MAX_PAGES}), stopping activity fetch");
        break;
      }

      let endpoint = format!("athlete/activities?after={after}&per_page={PER_PAGE}&page={page}");
      let response = self.strava_api_get(&endpoint)?;
      let body = response.text().map_err(|_| StravaError::StravaApiResponseMissingBody)?;
      log::debug!("JSON:\n{body}");

      let activities: Vec<SummaryActivity> =
        serde_json::from_str(&body).map_err(|_| {
                                     StravaError::StravaApiResponseDeserializationError(body)
                                   })?;

      if activities.is_empty() {
        log::debug!("No more activities on page {page}");
        break;
      }

      log::debug!("Page {page}: {} activities", activities.len());
      all_activities.extend(activities);
      page += 1;
    }

    self.cache.save("activities", &all_activities, Some(Self::ACTIVITIES_CACHE_TIME));

    // Sort newest-first (the API with `after` returns oldest-first)
    all_activities.sort_by(|a, b| {
                    b.start_date_local.as_deref().cmp(&a.start_date_local.as_deref())
                  });

    Ok(all_activities)
  }

  /// Whether the refresh token was updated during this session.
  pub fn token_refreshed(&self) -> bool {
    self.token_refreshed
  }

  /// Current refresh token (possibly updated after authentication).
  pub fn refresh_token(&self) -> &str {
    self.config.refresh_token()
  }

  /// Classify a reqwest error as NetworkUnavailable when it's a connectivity
  /// issue.
  fn classify_reqwest_error(e: reqwest::Error) -> StravaError {
    if e.is_connect() || e.is_timeout() {
      StravaError::NetworkUnavailable(e.to_string())
    } else if e.is_request() {
      // DNS resolution failures and similar come through as request errors
      let msg = e.to_string().to_lowercase();
      if msg.contains("dns") || msg.contains("resolve") || msg.contains("no address") {
        StravaError::NetworkUnavailable(e.to_string())
      } else {
        StravaError::StravaApiResponseError(e.to_string())
      }
    } else {
      StravaError::StravaApiResponseError(e.to_string())
    }
  }
}
