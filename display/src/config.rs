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
  // FIXME: Sleep interface and quiet hours should also move to PowerConfig
  /// Sleep interval between refreshes in seconds (default: 10800 = 3 hours)
  #[serde(default = "default_sleep_interval")]
  pub sleep_interval_secs: u64,

  /// Hour (0-23, local time) when the quiet period starts (default: 20)
  #[serde(default = "default_quiet_start")]
  pub quiet_start_hour: u32,

  /// Hour (0-23, local time) when the quiet period ends (default: 8)
  #[serde(default = "default_quiet_end")]
  pub quiet_end_hour: u32,

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

fn default_sleep_interval() -> u64 {
  10800
}
fn default_quiet_start() -> u32 {
  20
}
fn default_quiet_end() -> u32 {
  8
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
    Self { sleep_interval_secs:  default_sleep_interval(),
           quiet_start_hour:     default_quiet_start(),
           quiet_end_hour:       default_quiet_end(),
           goals:                default_goals(),
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
