//! Snapshot tests: render dashboard PNGs for every test config in `tests/`
//! and compare against reference images.
//!
//! Run with:  cargo test -p display --test snapshots
//!
//! To update reference snapshots after intentional visual changes:
//!   UPDATE_SNAPSHOTS=1 cargo test -p display --test snapshots
//!
//! Generated images are saved to `tmp/snapshots/` with the current git commit
//! hash in the filename for easy comparison across versions.

use display::config::DisplayConfig;
use display::renderer::{Scale, render_dashboard};
use image::RgbImage;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Minimal config wrapper — only the [display] section is needed for rendering.
#[derive(Debug, Deserialize)]
struct TestConfig {
  #[serde(default)]
  display: DisplayConfig,
}

/// One entry in fixtures.toml
#[derive(Debug, Deserialize)]
struct TestEntry {
  config:     String,
  athlete_id: u64,
}

/// Top-level fixtures.toml
#[derive(Debug, Deserialize)]
struct FixturesManifest {
  test: Vec<TestEntry>,
}

/// Wrapper matching the cache JSON format.
#[derive(Debug, Deserialize)]
struct CacheEntry<T> {
  data: T,
}

fn project_root() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn git_short_hash() -> String {
  Command::new("git").args(["rev-parse", "--short", "HEAD"])
                     .output()
                     .ok()
                     .and_then(|o| {
                       if o.status.success() {
                         Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
                       } else {
                         None
                       }
                     })
                     .unwrap_or_else(|| "unknown".to_string())
}

fn load_fixture<T: for<'de> serde::Deserialize<'de>>(fixture_dir: &Path, key: &str) -> T {
  let path = fixture_dir.join(format!("{key}.json"));
  let contents = fs::read_to_string(&path).unwrap_or_else(|e| {
                                            panic!("Failed to read fixture {}: {e}", path.display())
                                          });
  let entry: CacheEntry<T> = serde_json::from_str(&contents).unwrap_or_else(|e| {
                               panic!("Failed to parse fixture {}: {e}", path.display())
                             });
  entry.data
}

/// Compute the percentage of pixels that differ between two images.
/// Returns (diff_percentage, diff_image) where diff_image highlights changes.
fn image_diff(a: &RgbImage, b: &RgbImage) -> (f64, RgbImage) {
  assert_eq!(a.dimensions(), b.dimensions(), "Image dimensions must match");
  let (w, h) = a.dimensions();
  let mut diff_img = RgbImage::from_pixel(w, h, image::Rgb([255, 255, 255]));
  let mut diff_count = 0u64;
  let total = (w as u64) * (h as u64);

  for y in 0..h {
    for x in 0..w {
      let pa = a.get_pixel(x, y);
      let pb = b.get_pixel(x, y);
      if pa != pb {
        diff_count += 1;
        // Highlight diff in red
        diff_img.put_pixel(x, y, image::Rgb([255, 0, 0]));
      } else {
        // Dimmed version of original
        let r = (pa[0] as u16 + 255) / 2;
        let g = (pa[1] as u16 + 255) / 2;
        let b = (pa[2] as u16 + 255) / 2;
        diff_img.put_pixel(x, y, image::Rgb([r as u8, g as u8, b as u8]));
      }
    }
  }

  let pct = (diff_count as f64 / total as f64) * 100.0;
  (pct, diff_img)
}

fn render_for_entry(entry: &TestEntry) -> (String, RgbImage) {
  let root = project_root();
  let config_path = root.join("tests").join(&entry.config);
  let fixture_dir = root.join("tests/fixtures").join(entry.athlete_id.to_string());

  // Load config (only the [display] section matters for rendering)
  let config_str = fs::read_to_string(&config_path).unwrap_or_else(|e| {
                                                     panic!("Failed to read config {}: {e}",
                                                            config_path.display())
                                                   });
  let config: TestConfig = toml::from_str(&config_str).unwrap_or_else(|e| {
                                                        panic!("Failed to parse config {}: {e}",
                                                               config_path.display())
                                                      });

  // Load fixture data
  let athlete_stats: strava::types::AthleteStats = load_fixture(&fixture_dir, "stats");
  let activities: Vec<strava::types::SummaryActivity> = load_fixture(&fixture_dir, "activities");
  let athlete: strava::types::DetailedAthlete = load_fixture(&fixture_dir, "athlete");
  let avatar_bytes = fs::read(fixture_dir.join("avatar.img")).ok();

  let firstname = athlete.firstname.as_deref().unwrap_or("Athlete");
  let dashboard = strava::stats::compute(&athlete_stats, &activities, firstname, true, |sport| {
    config.display.longest_by_for(sport)
  });

  let scale = Scale::new(1);
  let img = render_dashboard(&dashboard,
                             None, // no battery in tests
                             &config.display,
                             avatar_bytes.as_deref(),
                             false, // not offline
                             scale);

  let name = entry.config.trim_end_matches(".toml").to_string();
  (name, img)
}

#[test]
fn snapshot_all_configs() {
  let root = project_root();
  let manifest_path = root.join("tests/fixtures.toml");
  let manifest_str = fs::read_to_string(&manifest_path).expect("Failed to read fixtures.toml");
  let manifest: FixturesManifest =
    toml::from_str(&manifest_str).expect("Failed to parse fixtures.toml");

  let commit = git_short_hash();
  let snapshot_dir = root.join("tmp/snapshots");
  let reference_dir = root.join("tests/snapshots");
  fs::create_dir_all(&snapshot_dir).expect("Failed to create snapshot dir");

  let update_snapshots = std::env::var("UPDATE_SNAPSHOTS").is_ok();
  if update_snapshots {
    fs::create_dir_all(&reference_dir).expect("Failed to create reference dir");
  }

  let mut failures: Vec<String> = Vec::new();

  for entry in &manifest.test {
    let (name, img) = render_for_entry(entry);

    // Save with commit hash
    let snapshot_path = snapshot_dir.join(format!("{name}_{commit}.png"));
    img.save(&snapshot_path)
       .unwrap_or_else(|e| panic!("Failed to save snapshot {}: {e}", snapshot_path.display()));

    // Also save as "latest" for easy viewing
    let latest_path = snapshot_dir.join(format!("{name}_latest.png"));
    img.save(&latest_path)
       .unwrap_or_else(|e| panic!("Failed to save latest {}: {e}", latest_path.display()));

    println!("Rendered: {}", snapshot_path.display());

    // Compare against reference if it exists
    let reference_path = reference_dir.join(format!("{name}.png"));
    if update_snapshots {
      img.save(&reference_path)
         .unwrap_or_else(|e| panic!("Failed to save reference {}: {e}", reference_path.display()));
      println!("Updated reference: {}", reference_path.display());
    } else if reference_path.exists() {
      let reference = image::open(&reference_path).unwrap_or_else(|e| {
                                                    panic!("Failed to open reference {}: \
                                                                    {e}",
                                                           reference_path.display())
                                                  })
                                                  .to_rgb8();

      if reference.dimensions() != img.dimensions() {
        failures.push(format!("{name}: dimension mismatch (reference {:?} vs rendered {:?})",
                              reference.dimensions(),
                              img.dimensions()));
        continue;
      }

      let (diff_pct, diff_img) = image_diff(&reference, &img);
      if diff_pct > 0.0 {
        let diff_path = snapshot_dir.join(format!("{name}_diff.png"));
        let _ = diff_img.save(&diff_path);
        failures.push(format!("{name}: {diff_pct:.2}% pixels differ (see {})",
                              diff_path.display()));
      } else {
        println!("{name}: OK (matches reference)");
      }
    } else {
      println!("{name}: no reference snapshot (run with UPDATE_SNAPSHOTS=1 to create)");
    }
  }

  if !failures.is_empty() {
    panic!("Snapshot differences detected:\n  {}", failures.join("\n  "));
  }
}
