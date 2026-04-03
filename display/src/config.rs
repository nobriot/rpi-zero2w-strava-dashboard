use common::SportType;
use serde::{Deserialize, Serialize};

/// A single sport distance goal for the dashboard progress bars.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GoalConfig {
  pub sport: SportType,
  pub km:    f64,
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
fn default_goals() -> Vec<GoalConfig> {
  vec![GoalConfig { sport: SportType::Run,
                    km:    800.0, },
       GoalConfig { sport: SportType::Ride,
                    km:    5000.0, },
       GoalConfig { sport: SportType::Swim,
                    km:    30.0, },]
}

impl Default for DisplayConfig {
  fn default() -> Self {
    Self { goals:                default_goals(),
           polyline_thickness:   default_polyline_thickness(),
           show_totals:          default_show_totals(),
           show_longest_fastest: default_show_longest_fastest(), }
  }
}

impl DisplayConfig {
  /// Look up the goal distance for a sport, if configured.
  pub fn goal_for(&self, sport: SportType) -> Option<f64> {
    self.goals.iter().find(|g| g.sport == sport).map(|g| g.km)
  }
}
