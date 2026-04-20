use common::{LongestBy, SportType};
use serde::{Deserialize, Serialize};

/// Display orientation.
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Orientation {
  #[default]
  Landscape,
  Portrait,
}

/// A single sport distance goal for the dashboard progress bars.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GoalConfig {
  pub sport: SportType,
  pub km:    f64,

  /// Whether the "longest" activity for this sport is ranked by distance
  /// (default) or by time. Default: "distance".
  #[serde(default)]
  pub longest_by: LongestBy,
}

/// Display and scheduling configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DisplayConfig {
  /// Ordered sport goals (1-3). Controls which progress bars appear and their
  /// order. First goal is always full-width; with 3 goals, 2nd and 3rd share
  /// a row.
  #[serde(default = "default_goals")]
  pub goals: Vec<GoalConfig>,

  /// Thickness of the polyline drawn for the last activity, in pixels
  /// (default: 4).
  #[serde(default = "default_polyline_thickness")]
  pub polyline_thickness: u32,

  /// Whether to show the TOTALS summary row (default: true).
  #[serde(default = "default_show_totals")]
  pub show_totals: bool,

  /// Whether to show the LONGEST / FASTEST section (default: true).
  #[serde(default = "default_show_longest_fastest")]
  pub show_longest_fastest: bool,

  /// Rotate the image 180 degrees. Useful when the display is mounted
  /// upside-down in a stand. Default: true.
  #[serde(default = "default_flip")]
  pub flip: bool,

  /// Show debug info (last sync timestamp) on the dashboard. Default: false.
  #[serde(default)]
  pub debug: bool,

  /// Display orientation. Default: landscape (800x480).
  /// Set to "portrait" for vertical layout (480x800).
  #[serde(default)]
  pub orientation: Orientation,

  /// Use lighter font weights (Regular/SemiBold/Bold instead of
  /// SemiBold/Bold/Black). Better for screen viewing. Default: false.
  #[serde(default)]
  pub light_fonts: bool,
}

fn default_polyline_thickness() -> u32 {
  4
}
fn default_show_totals() -> bool {
  true
}
fn default_show_longest_fastest() -> bool {
  true
}
fn default_flip() -> bool {
  true
}
fn default_goals() -> Vec<GoalConfig> {
  vec![GoalConfig { sport:      SportType::Run,
                    km:         800.0,
                    longest_by: LongestBy::default(), },
       GoalConfig { sport:      SportType::Ride,
                    km:         5000.0,
                    longest_by: LongestBy::default(), },
       GoalConfig { sport:      SportType::Swim,
                    km:         30.0,
                    longest_by: LongestBy::default(), },]
}

impl Default for DisplayConfig {
  fn default() -> Self {
    Self { goals:                default_goals(),
           polyline_thickness:   default_polyline_thickness(),
           show_totals:          default_show_totals(),
           show_longest_fastest: default_show_longest_fastest(),
           flip:                 default_flip(),
           debug:                false,
           orientation:          Orientation::default(),
           light_fonts:          false, }
  }
}

impl DisplayConfig {
  /// Look up the goal distance for a sport, if configured.
  pub fn goal_for(&self, sport: SportType) -> Option<f64> {
    self.goals.iter().find(|g| g.sport == sport).map(|g| g.km)
  }

  /// Look up how the "longest" activity is ranked for a sport.
  pub fn longest_by_for(&self, sport: SportType) -> LongestBy {
    self.goals.iter().find(|g| g.sport == sport).map(|g| g.longest_by).unwrap_or_default()
  }
}
