use serde::{Deserialize, Serialize};

/// Sport type enum shared across crates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SportType {
  Run,
  Ride,
  Swim,
}

/// How the "longest" activity is determined for a sport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LongestBy {
  #[default]
  Distance,
  Time,
}

/// Per-sport YTD summary (only present for sports with >= 1 activity this
/// year).
#[derive(Debug, Clone)]
pub struct SportSummary {
  pub sport:            SportType,
  pub ytd_distance_km:  f64,
  pub ytd_count:        u32,
  pub ytd_time_secs:    f64,
  pub ytd_time_display: String,
  pub fastest:          Option<ActivityHighlight>,
  pub longest:          Option<ActivityHighlight>,
}

/// Best effort for a standard running race distance.
/// Always present for each race bucket (5K, 10K, HM); fields are `None` when
/// no matching activity exists.
#[derive(Debug, Clone)]
pub struct RunRaceBest {
  pub label:               &'static str,
  pub target_km:           f64,
  pub distance_km:         Option<f64>,
  pub moving_time_display: Option<String>,
  pub pace:                Option<String>,
  pub name:                Option<String>,
  pub date:                Option<String>,
}

/// All the stats we want to display on the dashboard.
#[derive(Debug)]
pub struct DashboardStats {
  /// Per-sport summaries (only sports with >= 1 YTD activity)
  pub sports: Vec<SportSummary>,

  pub last_activity: Option<ActivityHighlight>,

  /// Athlete first name (for display header)
  pub athlete_first_name: String,

  /// Total number of activities (all types)
  pub activity_count:         usize,
  /// Total moving time across all activities in seconds
  pub total_moving_time_secs: u32,
  /// Total kudos across all activities this year
  pub total_kudos:            u32,
  /// Decoded polyline points (lat, lon) from the last activity
  pub last_activity_polyline: Vec<(f64, f64)>,

  /// Fastest run efforts at standard race distances (5K, 10K, HM)
  pub run_race_bests:         Vec<RunRaceBest>,
  /// Total YTD distance across all active sports in km
  pub total_distance_km:      f64,
  /// Total elevation gain across all activities in meters
  pub total_elevation_gain_m: f64,
  /// Include all sports in display even if zero activities (demo mode)
  pub show_all_sports:        bool,
}

/// A single activity highlighted for a specific reason (fastest, longest,
/// last).
#[derive(Debug, Clone)]
pub struct ActivityHighlight {
  pub sport:               SportType,
  pub name:                String,
  pub distance_km:         f64,
  pub moving_time_display: String,
  pub pace_or_speed:       String,
  pub date:                String,
  pub kudos:               u32,
}

impl DashboardStats {
  /// Number of activities
  pub fn activity_count(&self) -> usize {
    self.activity_count
  }

  /// Format total moving time as "Xd Yh Zm" (includes days when >= 24h)
  pub fn total_time_display(&self) -> String {
    let total_secs = self.total_moving_time_secs;
    let days = total_secs / 86400;
    let hours = (total_secs % 86400) / 3600;
    let minutes = (total_secs % 3600) / 60;
    if days > 0 {
      format!("{days}d {hours}h {minutes}m")
    } else {
      format!("{hours}h {minutes}m")
    }
  }

  /// Look up YTD distance for a given sport (0.0 if not active).
  pub fn ytd_distance_km(&self, sport: SportType) -> f64 {
    self.sports.iter().find(|s| s.sport == sport).map(|s| s.ytd_distance_km).unwrap_or(0.0)
  }

  pub fn print_summary(&self) {
    println!("╔═══════════════════════════════════════════════════════╗");
    println!("║              DASHBOARD STATS SUMMARY                  ║");
    println!("╚═══════════════════════════════════════════════════════╝\n");

    for s in &self.sports {
      let icon = match s.sport {
        SportType::Run => "🏃",
        SportType::Ride => "🚴",
        SportType::Swim => "🏊",
      };
      println!("{} YTD {:?}: {:.1} km · {} activities · {}",
               icon, s.sport, s.ytd_distance_km, s.ytd_count, s.ytd_time_display);
      if let Some(ref a) = s.fastest {
        println!("  ⚡ Fastest: \"{}\" — {:.1} km in {} ({})",
                 a.name, a.distance_km, a.moving_time_display, a.pace_or_speed);
      }
      if let Some(ref a) = s.longest {
        println!("  📏 Longest: \"{}\" — {:.1} km in {}",
                 a.name, a.distance_km, a.moving_time_display);
      }
    }
    println!();

    if !self.run_race_bests.is_empty() {
      println!("🏁 Running Race Bests:");
      for rb in &self.run_race_bests {
        if let Some(ref pace) = rb.pace {
          println!("  {} {} — \"{}\" ({})",
                   rb.label,
                   pace,
                   rb.name.as_deref().unwrap_or("—"),
                   rb.date.as_deref().unwrap_or("—"));
        } else {
          println!("  {} —", rb.label);
        }
      }
      println!();
    }

    println!("📊 Totals: {:.1} km · {} activities · {} · {:.0}m ↑ · {} kudos",
             self.total_distance_km,
             self.activity_count,
             self.total_time_display(),
             self.total_elevation_gain_m,
             self.total_kudos);
    println!();

    if let Some(ref a) = self.last_activity {
      println!("🕐 Last Activity: \"{}\" — {:.1} km on {}", a.name, a.distance_km, a.date);
    }
  }
}
