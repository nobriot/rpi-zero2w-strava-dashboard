use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Default cache TTL: 3 hours
const DEFAULT_MAX_AGE_SECS: u64 = 3 * 3600;

/// Wrapper that stores fetched data alongside a timestamp.
#[derive(Debug, Serialize, Deserialize)]
struct CacheEntry<T> {
  fetched_at: u64,
  data:       T,
  max_age:    u64,
}

/// File-based JSON cache stored in ~/.cache/rpi-zero2w-strava-dash/
pub struct Cache {
  dir:             PathBuf,
  default_max_age: u64,
}

impl Cache {
  pub fn new() -> Self {
    let dir = dirs::cache_dir().unwrap_or_else(|| PathBuf::from(".cache"))
                               .join("rpi-zero2w-strava-dash");

    Self { dir,
           default_max_age: DEFAULT_MAX_AGE_SECS }
  }

  /// Delete the entire cache directory.
  pub fn clear(&self) -> Result<(), String> {
    if self.dir.exists() {
      fs::remove_dir_all(&self.dir).map_err(|e| {
                                     format!("Failed to remove cache dir {}: {e}",
                                             self.dir.display())
                                   })?;
      log::info!("Cache cleared: {}", self.dir.display());
    } else {
      log::info!("Cache directory does not exist, nothing to clear");
    }
    Ok(())
  }

  /// Set a custom TTL (in seconds) for cache freshness checks.
  pub fn with_max_age(mut self, seconds: u64) -> Self {
    self.default_max_age = seconds;
    self
  }

  fn path(&self, key: &str) -> PathBuf {
    self.dir.join(format!("{key}.json"))
  }

  fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
  }

  /// Load a cached value if it exists and is still fresh.
  pub fn load<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Option<T> {
    let path = self.path(key);
    let contents = fs::read_to_string(&path).ok()?;
    let entry: CacheEntry<T> = serde_json::from_str(&contents).ok()?;

    let age = Self::now().saturating_sub(entry.fetched_at);
    if age > entry.max_age {
      log::info!("Cache entry '{key}' expired ({age}s old, max {0}s)", entry.max_age);
      return None;
    }

    log::info!("Cache hit for '{key}' ({age}s old)");
    Some(entry.data)
  }

  /// Load a cached value even if it has expired.
  /// Returns `None` only if the file doesn't exist or can't be deserialized.
  pub fn load_stale<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Option<T> {
    let path = self.path(key);
    let contents = fs::read_to_string(&path).ok()?;
    let entry: CacheEntry<T> = serde_json::from_str(&contents).ok()?;

    let age = Self::now().saturating_sub(entry.fetched_at);
    log::info!("Stale cache hit for '{key}' ({age}s old)");
    Some(entry.data)
  }

  /// Save a value to the cache.
  pub fn save<T: Serialize>(&self, key: &str, data: &T, max_age: Option<u64>) {
    if let Err(e) = fs::create_dir_all(&self.dir) {
      log::warn!("Failed to create cache directory: {e}");
      return;
    }

    let entry_max_age = if let Some(a) = max_age { a } else { self.default_max_age };
    let entry = CacheEntry { fetched_at: Self::now(),
                             data,
                             max_age: entry_max_age };

    match serde_json::to_string_pretty(&entry) {
      Ok(json) => {
        if let Err(e) = fs::write(self.path(key), &json) {
          log::warn!("Failed to write cache file '{key}': {e}");
        } else {
          log::info!("Cached '{key}'");
        }
      },
      Err(e) => log::warn!("Failed to serialize cache entry '{key}': {e}"),
    }
  }
}

impl Default for Cache {
  fn default() -> Self {
    Self::new()
  }
}
