use serde::{Deserialize, Serialize};

/// Athlete statistics from Strava API
/// Endpoint: GET /athletes/{id}/stats
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AthleteStats {
  /// The recent (last 4 weeks) ride totals
  #[serde(default)]
  pub recent_ride_totals: Option<ActivityTotal>,

  /// The recent (last 4 weeks) run totals
  #[serde(default)]
  pub recent_run_totals: Option<ActivityTotal>,

  /// The recent (last 4 weeks) swim totals
  #[serde(default)]
  pub recent_swim_totals: Option<ActivityTotal>,

  /// The year to date ride totals
  #[serde(default)]
  pub ytd_ride_totals: Option<ActivityTotal>,

  /// The year to date run totals
  #[serde(default)]
  pub ytd_run_totals: Option<ActivityTotal>,

  /// The year to date swim totals
  #[serde(default)]
  pub ytd_swim_totals: Option<ActivityTotal>,

  /// The all time ride totals
  #[serde(default)]
  pub all_ride_totals: Option<ActivityTotal>,

  /// The all time run totals
  #[serde(default)]
  pub all_run_totals: Option<ActivityTotal>,

  /// The all time swim totals
  #[serde(default)]
  pub all_swim_totals: Option<ActivityTotal>,
}

/// Activity totals for a specific sport type and time period
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ActivityTotal {
  /// The number of activities
  pub count: u32,

  /// The total distance in meters
  pub distance: f64,

  /// The total moving time in seconds
  pub moving_time: f64,

  /// The total elapsed time in seconds
  pub elapsed_time: f64,

  /// The total elevation gain in meters
  pub elevation_gain: f64,

  /// The number of achievements (not always present)
  #[serde(default)]
  pub achievement_count: Option<u32>,
}

/// Sport type enum for querying stats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SportType {
  Run,
  Ride,
  Swim,
}

/// Time period enum for querying stats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Period {
  Recent,     // Last 4 weeks
  YearToDate, // Current year
  AllTime,    // All time
}

impl ActivityTotal {
  /// Get distance in kilometers
  pub fn distance_km(&self) -> f64 {
    self.distance / 1000.0
  }

  /// Get distance in miles
  pub fn distance_miles(&self) -> f64 {
    self.distance / 1609.34
  }

  /// Get moving time as hours
  pub fn moving_time_hours(&self) -> f64 {
    self.moving_time / 3600.0
  }

  /// Get elapsed time as hours
  pub fn elapsed_time_hours(&self) -> f64 {
    self.elapsed_time / 3600.0
  }

  /// Get elevation gain in feet
  pub fn elevation_gain_feet(&self) -> f64 {
    self.elevation_gain * 3.28084
  }

  /// Get average pace in minutes per kilometer
  pub fn avg_pace_min_per_km(&self) -> Option<f64> {
    if self.distance > 0.0 {
      Some((self.moving_time / 60.0) / (self.distance / 1000.0))
    } else {
      None
    }
  }

  /// Get average pace in minutes per mile
  pub fn avg_pace_min_per_mile(&self) -> Option<f64> {
    if self.distance > 0.0 {
      Some((self.moving_time / 60.0) / (self.distance / 1609.34))
    } else {
      None
    }
  }

  /// Get average speed in km/h
  pub fn avg_speed_kmh(&self) -> Option<f64> {
    if self.moving_time > 0.0 {
      Some((self.distance / 1000.0) / (self.moving_time / 3600.0))
    } else {
      None
    }
  }

  /// Get average speed in mph
  pub fn avg_speed_mph(&self) -> Option<f64> {
    if self.moving_time > 0.0 {
      Some((self.distance / 1609.34) / (self.moving_time / 3600.0))
    } else {
      None
    }
  }

  /// Format moving time as "Xh Ym Zs"
  pub fn format_moving_time(&self) -> String {
    let hours = (self.moving_time / 3600.0) as usize;
    let minutes = (self.moving_time % 3600.0) as usize / 60;
    let seconds = self.moving_time % 60.0;

    if hours > 0 {
      format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
      format!("{}m {}s", minutes, seconds)
    } else {
      format!("{}s", seconds)
    }
  }

  /// Format pace as "M:SS /km"
  pub fn format_pace_per_km(&self) -> String {
    if let Some(pace) = self.avg_pace_min_per_km() {
      let minutes = pace.floor() as u32;
      let seconds = ((pace - minutes as f64) * 60.0).round() as u32;
      format!("{}:{:02} /km", minutes, seconds)
    } else {
      "--:-- /km".to_string()
    }
  }

  /// Format pace as "M:SS /mi"
  pub fn format_pace_per_mile(&self) -> String {
    if let Some(pace) = self.avg_pace_min_per_mile() {
      let minutes = pace.floor() as u32;
      let seconds = ((pace - minutes as f64) * 60.0).round() as u32;
      format!("{}:{:02} /mi", minutes, seconds)
    } else {
      "--:-- /mi".to_string()
    }
  }
}

impl AthleteStats {
  /// Get totals for a specific sport type and period
  pub fn get_totals(&self, sport: SportType, period: Period) -> Option<&ActivityTotal> {
    match (sport, period) {
      (SportType::Ride, Period::Recent) => self.recent_ride_totals.as_ref(),
      (SportType::Ride, Period::YearToDate) => self.ytd_ride_totals.as_ref(),
      (SportType::Ride, Period::AllTime) => self.all_ride_totals.as_ref(),
      (SportType::Run, Period::Recent) => self.recent_run_totals.as_ref(),
      (SportType::Run, Period::YearToDate) => self.ytd_run_totals.as_ref(),
      (SportType::Run, Period::AllTime) => self.all_run_totals.as_ref(),
      (SportType::Swim, Period::Recent) => self.recent_swim_totals.as_ref(),
      (SportType::Swim, Period::YearToDate) => self.ytd_swim_totals.as_ref(),
      (SportType::Swim, Period::AllTime) => self.all_swim_totals.as_ref(),
    }
  }

  /// Get year to date totals for a sport
  pub fn ytd_totals(&self, sport: SportType) -> Option<&ActivityTotal> {
    self.get_totals(sport, Period::YearToDate)
  }

  /// Get recent (last 4 weeks) totals for a sport
  pub fn recent_totals(&self, sport: SportType) -> Option<&ActivityTotal> {
    self.get_totals(sport, Period::Recent)
  }

  /// Get all time totals for a sport
  pub fn all_time_totals(&self, sport: SportType) -> Option<&ActivityTotal> {
    self.get_totals(sport, Period::AllTime)
  }

  /// Print a summary of all stats
  pub fn print_summary(&self) {
    println!("╔═══════════════════════════════════════════════════════╗");
    println!("║              ATHLETE STATISTICS SUMMARY               ║");
    println!("╚═══════════════════════════════════════════════════════╝\n");

    self.print_sport_summary(SportType::Run);
    self.print_sport_summary(SportType::Ride);
    self.print_sport_summary(SportType::Swim);
  }

  fn print_sport_summary(&self, sport: SportType) {
    let sport_name = match sport {
      SportType::Run => "🏃 RUNNING",
      SportType::Ride => "🚴 CYCLING",
      SportType::Swim => "🏊 SWIMMING",
    };

    println!("{}", sport_name);
    println!("─────────────────────────────────────────────────────────");

    if let Some(ytd) = self.ytd_totals(sport) {
      println!("  Year to Date:");
      println!("    Activities: {}", ytd.count);
      println!("    Distance:   {:.1} km", ytd.distance_km());
      println!("    Time:       {}", ytd.format_moving_time());
      println!("    Elevation:  {:.0} m", ytd.elevation_gain);

      if matches!(sport, SportType::Run) {
        println!("    Avg Pace:   {}", ytd.format_pace_per_km());
      } else if matches!(sport, SportType::Ride)
                && let Some(speed) = ytd.avg_speed_kmh()
      {
        println!("    Avg Speed:  {:.1} km/h", speed);
      }
    } else {
      println!("  Year to Date: No data");
    }

    if let Some(recent) = self.recent_totals(sport) {
      println!("\n  Recent (4 weeks):");
      println!("    Activities: {}", recent.count);
      println!("    Distance:   {:.1} km", recent.distance_km());
      println!("    Time:       {}", recent.format_moving_time());
    } else {
      println!("\n  Recent (4 weeks): No data");
    }

    if let Some(all_time) = self.all_time_totals(sport) {
      println!("\n  All Time:");
      println!("    Activities: {}", all_time.count);
      println!("    Distance:   {:.1} km", all_time.distance_km());
      println!("    Time:       {}", all_time.format_moving_time());
    }

    println!();
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_deserialize_athlete_stats() {
    let json = r#"{
            "recent_ride_totals": {
                "count": 5,
                "distance": 50000.0,
                "moving_time": 7200,
                "elapsed_time": 8000,
                "elevation_gain": 500.0,
                "achievement_count": 2
            },
            "recent_run_totals": {
                "count": 10,
                "distance": 50000.0,
                "moving_time": 14400,
                "elapsed_time": 15000,
                "elevation_gain": 300.0
            },
            "ytd_ride_totals": {
                "count": 50,
                "distance": 500000.0,
                "moving_time": 72000,
                "elapsed_time": 80000,
                "elevation_gain": 5000.0
            },
            "ytd_run_totals": {
                "count": 100,
                "distance": 500000.0,
                "moving_time": 144000,
                "elapsed_time": 150000,
                "elevation_gain": 3000.0
            },
            "all_ride_totals": {
                "count": 200,
                "distance": 2000000.0,
                "moving_time": 288000,
                "elapsed_time": 300000,
                "elevation_gain": 20000.0
            },
            "all_run_totals": {
                "count": 500,
                "distance": 2500000.0,
                "moving_time": 720000,
                "elapsed_time": 750000,
                "elevation_gain": 15000.0
            }
        }"#;

    let stats: AthleteStats = serde_json::from_str(json).unwrap();

    let recent_run = stats.recent_totals(SportType::Run).unwrap();
    assert_eq!(recent_run.count, 10);
    assert_eq!(recent_run.distance_km(), 50.0);
    assert_eq!(recent_run.moving_time_hours(), 4.0);

    let ytd_ride = stats.ytd_totals(SportType::Ride).unwrap();
    assert_eq!(ytd_ride.count, 50);
    assert_eq!(ytd_ride.distance_km(), 500.0);

    let all_run = stats.all_time_totals(SportType::Run).unwrap();
    assert_eq!(all_run.count, 500);
    assert_eq!(all_run.distance_km(), 2500.0);
  }

  #[test]
  fn test_activity_total_conversions() {
    let total = ActivityTotal { count:             10,
                                distance:          10000.0, // 10 km
                                moving_time:       3600.0,  // 1 hour
                                elapsed_time:      3700.0,
                                elevation_gain:    100.0,
                                achievement_count: Some(5), };

    assert_eq!(total.distance_km(), 10.0);
    assert!((total.distance_miles() - 6.2137).abs() < 0.01);
    assert_eq!(total.moving_time_hours(), 1.0);
    assert!((total.elevation_gain_feet() - 328.08).abs() < 0.1);

    let pace_km = total.avg_pace_min_per_km().unwrap();
    assert!((pace_km - 6.0).abs() < 0.01);

    let speed_kmh = total.avg_speed_kmh().unwrap();
    assert!((speed_kmh - 10.0).abs() < 0.01);
  }

  #[test]
  fn test_format_functions() {
    let total = ActivityTotal { count:             1,
                                distance:          5000.0, // 5 km
                                moving_time:       1500.0, // 25 minutes
                                elapsed_time:      1600.0,
                                elevation_gain:    50.0,
                                achievement_count: None, };

    assert_eq!(total.format_moving_time(), "25m 0s");
    assert_eq!(total.format_pace_per_km(), "5:00 /km");

    let long_total = ActivityTotal { count:             1,
                                     distance:          100000.0,
                                     moving_time:       14523.0, // 4h 2m 3s
                                     elapsed_time:      15000.0,
                                     elevation_gain:    1000.0,
                                     achievement_count: None, };

    assert_eq!(long_total.format_moving_time(), "4h 2m 3s");
  }

  #[test]
  fn test_zero_distance() {
    let total = ActivityTotal { count:             0,
                                distance:          0.0,
                                moving_time:       0.0,
                                elapsed_time:      0.0,
                                elevation_gain:    0.0,
                                achievement_count: None, };

    assert!(total.avg_pace_min_per_km().is_none());
    assert!(total.avg_speed_kmh().is_none());
    assert_eq!(total.format_pace_per_km(), "--:-- /km");
  }
}
