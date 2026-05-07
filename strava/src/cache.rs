use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
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

/// File-based JSON cache stored in ~/.cache/rpi-zero2w-strava-dashboard/
///
/// `root` always points at the package cache directory. `dir` is the working
/// directory for I/O -- equal to `root` for a top-level Cache, or `root/<id>`
/// for a per-athlete scope. Keeping `root` separate makes `for_athlete`
/// idempotent: calling it repeatedly never compounds nested `/<id>/<id>/...`
/// segments.
pub struct Cache {
  root:            PathBuf,
  dir:             PathBuf,
  default_max_age: u64,
}

impl Cache {
  pub fn new() -> Self {
    let root =
      dirs::cache_dir().unwrap_or_else(|| PathBuf::from(".cache")).join(env!("CARGO_PKG_NAME"));

    Self { dir: root.clone(),
           root,
           default_max_age: DEFAULT_MAX_AGE_SECS }
  }

  /// Return a new Cache scoped to a per-athlete subdirectory.
  ///
  /// Always derived from the root cache directory, so calling this on an
  /// already-scoped Cache does not nest another `/<id>` segment.
  pub fn for_athlete(&self, athlete_id: u64) -> Self {
    Self { root:            self.root.clone(),
           dir:             self.root.join(athlete_id.to_string()),
           default_max_age: self.default_max_age, }
  }

  /// The cache directory path (per-athlete after `for_athlete`).
  pub fn dir(&self) -> &Path {
    &self.dir
  }

  /// Delete the entire root cache directory.
  pub fn clear(&self) -> Result<(), String> {
    if self.root.exists() {
      fs::remove_dir_all(&self.root).map_err(|e| {
                                      format!("Failed to remove cache dir {}: {e}",
                                              self.root.display())
                                    })?;
      log::debug!("Cache cleared: {}", self.root.display());
    } else {
      log::debug!("Cache directory does not exist, nothing to clear");
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
      log::debug!("Cache entry '{key}' expired ({age}s old, max {0}s)", entry.max_age);
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
      log::warn!("Failed to create cache directory {}: {e}", self.dir.display());
      return;
    }

    let entry_max_age = if let Some(a) = max_age { a } else { self.default_max_age };
    let entry = CacheEntry { fetched_at: Self::now(),
                             data,
                             max_age: entry_max_age };

    let path = self.path(key);
    match serde_json::to_string_pretty(&entry) {
      Ok(json) => {
        if let Err(e) = fs::write(&path, &json) {
          log::warn!("Failed to write cache file {}: {e}", path.display());
        } else {
          log::debug!("Cached '{key}' -> {} ({} bytes)", path.display(), json.len());
        }
      },
      Err(e) => log::warn!("Failed to serialize cache entry '{key}': {e}"),
    }
  }

  /// Scan for per-athlete subdirectories and return the most recently used
  /// one (by modification time of cached files). Returns `None` if no
  /// per-athlete subdirectories exist.
  pub fn most_recent_athlete_cache(&self) -> Option<Self> {
    let entries = fs::read_dir(&self.dir).ok()?;
    let mut best: Option<(PathBuf, SystemTime)> = None;

    for entry in entries.flatten() {
      let path = entry.path();
      if !path.is_dir() {
        continue;
      }
      // Only consider numeric subdirectories (athlete IDs)
      let is_athlete_dir =
        path.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.parse::<u64>().is_ok());
      if !is_athlete_dir {
        continue;
      }

      let mtime = ["stats.json", "activities.json", "athlete.json"].iter()
                                                                    .filter_map(|f| {
                                                                      fs::metadata(path.join(f)).and_then(|m| m.modified()).ok()
                                                                    })
                                                                    .max()
                                                                    .unwrap_or(UNIX_EPOCH);

      if best.as_ref().is_none_or(|(_, t)| mtime > *t) {
        best = Some((path, mtime));
      }
    }

    best.map(|(path, _)| Self { root:            self.root.clone(),
                                dir:             path,
                                default_max_age: self.default_max_age, })
  }
}

impl Default for Cache {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn at(root: &Path) -> Cache {
    Cache { root:            root.to_path_buf(),
            dir:             root.to_path_buf(),
            default_max_age: DEFAULT_MAX_AGE_SECS, }
  }

  #[test]
  fn for_athlete_is_idempotent() {
    // Repeated for_athlete() calls must not nest /<id>/<id>/... segments.
    // Regression: the path used to grow on every fetch cycle until it hit
    // PATH_MAX, surfacing as ENAMETOOLONG when create_dir_all() ran.
    let root = PathBuf::from("/tmp/cache-test-root");
    let c0 = at(&root);
    let c1 = c0.for_athlete(12345);
    let c2 = c1.for_athlete(12345);
    let c3 = c2.for_athlete(67890);
    assert_eq!(c1.dir(), root.join("12345"));
    assert_eq!(c2.dir(), root.join("12345"));
    assert_eq!(c3.dir(), root.join("67890"));
  }

  #[test]
  fn most_recent_preserves_root() {
    let root = PathBuf::from("/tmp/cache-test-root");
    let c0 = at(&root);
    // Manually craft a "selected" cache as if returned by most_recent_athlete_cache
    let scoped = Cache { root:            c0.root.clone(),
                         dir:             root.join("999"),
                         default_max_age: c0.default_max_age, };
    // for_athlete on a previously-selected cache must still derive from root
    assert_eq!(scoped.for_athlete(42).dir(), root.join("42"));
  }
}
