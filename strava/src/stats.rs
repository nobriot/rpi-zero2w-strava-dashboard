use crate::types::{AthleteStats, SummaryActivity};
use common::{ActivityHighlight, DashboardStats, LongestBy, RunRaceBest, SportSummary, SportType};

/// Compute dashboard stats from aggregate athlete stats and the list of
/// individual activities fetched for the current year.
///
/// `longest_by_for` determines whether the "longest" activity per sport is
/// ranked by distance or time.
pub fn compute(stats: &AthleteStats,
               activities: &[SummaryActivity],
               athlete_first_name: &str,
               show_all_sports: bool,
               longest_by_for: impl Fn(SportType) -> LongestBy)
               -> DashboardStats {
  let all_sport_types = [SportType::Run, SportType::Ride, SportType::Swim];

  let sports: Vec<SportSummary> =
    all_sport_types.iter()
                   .filter_map(|&sport| {
                     let ytd = stats.ytd_totals(sport);
                     let count = ytd.map(|t| t.count).unwrap_or(0);
                     if count == 0 && !show_all_sports {
                       return None;
                     }

                     let sport_activities: Vec<&SummaryActivity> =
                       activities.iter()
                                 .filter(|a| a.sport() == Some(sport) && a.can_be_displayed())
                                 .collect();

                     let fastest = sport_activities.iter()
                                                   .filter(|a| a.distance > 0.0)
                                                   .max_by(|a, b| {
                                                     a.average_speed
                                                      .partial_cmp(&b.average_speed)
                                                      .unwrap_or(std::cmp::Ordering::Equal)
                                                   })
                                                   .map(|a| to_highlight(a, sport));

                     let longest =
                       sport_activities.iter()
                                       .max_by(|a, b| match longest_by_for(sport) {
                                         LongestBy::Distance => {
                                           a.distance
                                            .partial_cmp(&b.distance)
                                            .unwrap_or(std::cmp::Ordering::Equal)
                                         },
                                         LongestBy::Time => a.moving_time.cmp(&b.moving_time),
                                       })
                                       .map(|a| to_highlight(a, sport));

                     let (distance_km, moving_time, time_display) = match ytd {
                       Some(t) => {
                         (t.distance_km(), t.moving_time, format_duration_secs(t.moving_time))
                       },
                       None => (0.0, 0.0, "0h 0m".to_string()),
                     };

                     Some(SportSummary { sport,
                                         ytd_distance_km: distance_km,
                                         ytd_count: count,
                                         ytd_time_secs: moving_time,
                                         ytd_time_display: time_display,
                                         fastest,
                                         longest })
                   })
                   .collect();

  // Last activity = most recent non-commute, public activity by date
  let last_eligible = activities.iter().filter(|a| a.can_be_displayed()).max_by(|a, b| {
                                                                          a.start_date_local
                                                                           .as_deref()
                                                                           .cmp(&b.start_date_local
                                                                                  .as_deref())
                                                                        });
  let last_activity = last_eligible.map(|a| {
                                     let sport = a.sport().unwrap_or(SportType::Ride);
                                     to_highlight(a, sport)
                                   });

  let activity_count = activities.len();
  let total_moving_time_secs: u32 = activities.iter().map(|a| a.moving_time).sum();
  let total_kudos: u32 = activities.iter().map(|a| a.kudos_count).sum();
  let total_elevation_gain_m: f64 = activities.iter().map(|a| a.total_elevation_gain).sum();
  let last_activity_polyline = last_eligible.map(|a| a.polyline_points()).unwrap_or_default();

  let total_distance_km: f64 = sports.iter().map(|s| s.ytd_distance_km).sum();
  let run_race_bests = compute_race_bests(activities);

  DashboardStats { sports,
                   last_activity,
                   athlete_first_name: athlete_first_name.to_string(),
                   activity_count,
                   total_moving_time_secs,
                   total_kudos,
                   last_activity_polyline,
                   run_race_bests,
                   total_distance_km,
                   total_elevation_gain_m,
                   show_all_sports }
}

fn format_date(raw: &Option<String>) -> String {
  raw.as_deref().unwrap_or("").get(..10).unwrap_or("").to_string()
}

fn format_duration_secs(secs: f64) -> String {
  let total_hours = (secs / 3600.0) as u32;
  let days = total_hours / 24;
  let hours = total_hours % 24;
  let minutes = ((secs % 3600.0) / 60.0) as u32;
  if days > 0 {
    format!("{days}d {hours}h {minutes:02}m")
  } else {
    format!("{hours}h {minutes:02}m")
  }
}

fn to_highlight(a: &SummaryActivity, sport: SportType) -> ActivityHighlight {
  let pace_or_speed = match sport {
    SportType::Run => a.format_pace_per_km(),
    SportType::Ride => format!("{:.1} km/h", a.avg_speed_kmh()),
    SportType::Swim => a.format_pace_per_100m(),
    SportType::WeightTraining => String::new(),
  };
  ActivityHighlight { sport,
                      name: a.name.clone().unwrap_or_else(|| "Unnamed".to_string()),
                      distance_km: a.distance_km(),
                      moving_time_display: a.format_moving_time(),
                      pace_or_speed,
                      date: format_date(&a.start_date_local),
                      kudos: a.kudos_count,
                      is_mtb: a.is_mtb() }
}

/// Race distance buckets for finding best running efforts.
const RACE_BUCKETS: &[(&str, f64, f64)] = &[// (label, target_km, min_km)
                                            ("5K", 5.0, 4.95),
                                            ("10K", 10.0, 9.9),
                                            ("HM", 21.1, 21.0)];

fn compute_race_bests(activities: &[SummaryActivity]) -> Vec<RunRaceBest> {
  let runs: Vec<&SummaryActivity> =
    activities.iter()
              .filter(|a| a.is_run() && a.distance > 0.0 && a.moving_time > 0)
              .collect();

  RACE_BUCKETS.iter()
              .map(|&(label, target_km, min_km)| {
                let best = runs.iter().filter(|a| a.distance_km() >= min_km).min_by(|a, b| {
        let est_a = a.moving_time as f64 * target_km / a.distance_km();
        let est_b = b.moving_time as f64 * target_km / b.distance_km();
        est_a.partial_cmp(&est_b).unwrap_or(std::cmp::Ordering::Equal)
      });

                match best {
                  Some(b) => {
                    let estimated_secs = b.moving_time as f64 * target_km / b.distance_km();
                    let est_time = format_estimated_time(estimated_secs);
                    let total_pace_secs = (estimated_secs / target_km).round() as u32;
                    let pace_whole = total_pace_secs / 60;
                    let pace_frac = total_pace_secs % 60;
                    let pace = format!("{pace_whole}:{pace_frac:02} /km");

                    RunRaceBest { label,
                                  target_km,
                                  distance_km: Some(b.distance_km()),
                                  moving_time_display: Some(est_time),
                                  pace: Some(pace),
                                  name: Some(b.name
                                              .clone()
                                              .unwrap_or_else(|| "Unnamed".to_string())),
                                  date: Some(format_date(&b.start_date_local)) }
                  },
                  None => RunRaceBest { label,
                                        target_km,
                                        distance_km: None,
                                        moving_time_display: None,
                                        pace: None,
                                        name: None,
                                        date: None },
                }
              })
              .collect()
}

/// Format an estimated time in seconds as "Xh Ym Zs" or "Ym Zs".
fn format_estimated_time(secs: f64) -> String {
  let total = secs.round() as u32;
  let hours = total / 3600;
  let minutes = (total % 3600) / 60;
  let seconds = total % 60;
  if hours > 0 {
    format!("{hours}h {minutes:02}m {seconds:02}s")
  } else if minutes > 0 {
    format!("{minutes}m {seconds:02}s")
  } else {
    format!("{seconds}s")
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::types::SummaryActivity;

  fn make_run(distance_m: f64, moving_time: u32, name: &str, date: &str) -> SummaryActivity {
    SummaryActivity { id: 0,
                      name: Some(name.to_string()),
                      activity_type: Some("Run".to_string()),
                      sport_type: Some("Run".to_string()),
                      distance: distance_m,
                      moving_time,
                      elapsed_time: moving_time,
                      total_elevation_gain: 0.0,
                      average_speed: if moving_time > 0 {
                        distance_m / moving_time as f64
                      } else {
                        0.0
                      },
                      max_speed: 0.0,
                      start_date: Some(format!("{date}T08:00:00Z")),
                      start_date_local: Some(format!("{date}T08:00:00Z")),
                      map: None,
                      commute: false,
                      private: false,
                      kudos_count: 0 }
  }

  #[test]
  fn test_longer_faster_run_beats_in_bucket_run() {
    let in_bucket = make_run(10_500.0, 3000, "Short run", "2026-03-08");
    let longer = make_run(13_000.0, 3300, "Interval session", "2026-03-10");

    let results = compute_race_bests(&[in_bucket, longer]);
    let ten_k = results.iter().find(|r| r.label == "10K").unwrap();

    assert_eq!(ten_k.name.as_deref(), Some("Interval session"));
  }

  #[test]
  fn test_exact_target_distance() {
    let exact = make_run(10_000.0, 3000, "Exact 10K", "2026-03-01");
    let results = compute_race_bests(&[exact]);
    let ten_k = results.iter().find(|r| r.label == "10K").unwrap();

    assert_eq!(ten_k.name.as_deref(), Some("Exact 10K"));
    assert_eq!(ten_k.moving_time_display.as_deref(), Some("50m 00s"));
  }

  #[test]
  fn test_run_below_target_excluded() {
    let short = make_run(9_000.0, 2400, "Short run", "2026-03-01");
    let results = compute_race_bests(&[short]);
    let ten_k = results.iter().find(|r| r.label == "10K").unwrap();

    assert!(ten_k.name.is_none(), "9km run should not qualify as 10K best");
  }

  #[test]
  fn test_non_run_excluded() {
    let ride = SummaryActivity { activity_type: Some("Ride".to_string()),
                                 sport_type: Some("Ride".to_string()),
                                 ..make_run(15_000.0, 2000, "Fast ride", "2026-03-01") };
    let results = compute_race_bests(&[ride]);
    let ten_k = results.iter().find(|r| r.label == "10K").unwrap();

    assert!(ten_k.name.is_none());
  }

  #[test]
  fn test_estimated_time_display() {
    let run = make_run(12_000.0, 3600, "Long run", "2026-03-01");
    let results = compute_race_bests(&[run]);
    let ten_k = results.iter().find(|r| r.label == "10K").unwrap();

    assert_eq!(ten_k.moving_time_display.as_deref(), Some("50m 00s"));
    assert_eq!(ten_k.pace.as_deref(), Some("5:00 /km"));
  }

  fn make_ride(distance_m: f64, moving_time: u32, name: &str, date: &str) -> SummaryActivity {
    SummaryActivity { activity_type: Some("Ride".to_string()),
                      sport_type: Some("Ride".to_string()),
                      ..make_run(distance_m, moving_time, name, date) }
  }

  fn compute_with(activities: &[SummaryActivity]) -> DashboardStats {
    let stats = crate::types::AthleteStats::default();
    compute(&stats, activities, "Test", false, |_| LongestBy::default())
  }

  #[test]
  fn test_last_activity_picks_most_recent() {
    let old = make_run(5_000.0, 1500, "January run", "2026-01-01");
    let recent = make_run(10_000.0, 3000, "March run", "2026-03-25");
    let mid = make_run(7_000.0, 2000, "February run", "2026-02-15");

    let stats = compute_with(&[old, mid, recent]);
    assert_eq!(stats.last_activity.as_ref().unwrap().name, "March run");
  }

  #[test]
  fn test_last_activity_picks_most_recent_newest_first() {
    let old = make_run(5_000.0, 1500, "January run", "2026-01-01");
    let recent = make_run(10_000.0, 3000, "March run", "2026-03-25");

    let stats = compute_with(&[recent, old]);
    assert_eq!(stats.last_activity.as_ref().unwrap().name, "March run");
  }

  #[test]
  fn test_last_activity_skips_commute() {
    let commute = SummaryActivity { commute: true,
                                    ..make_ride(5_000.0, 900, "Bike commute", "2026-03-25") };
    let regular = make_run(10_000.0, 3000, "Morning run", "2026-03-20");

    let stats = compute_with(&[regular, commute]);
    assert_eq!(stats.last_activity.as_ref().unwrap().name, "Morning run");
  }

  #[test]
  fn test_last_activity_skips_private() {
    let private = SummaryActivity { private: true,
                                    ..make_run(8_000.0, 2400, "Secret run", "2026-03-25") };
    let public = make_run(5_000.0, 1500, "Public run", "2026-03-20");

    let stats = compute_with(&[public, private]);
    assert_eq!(stats.last_activity.as_ref().unwrap().name, "Public run");
  }

  #[test]
  fn test_last_activity_none_when_all_filtered() {
    let commute = SummaryActivity { commute: true,
                                    ..make_ride(5_000.0, 900, "Commute", "2026-03-25") };
    let private = SummaryActivity { private: true,
                                    ..make_run(8_000.0, 2400, "Private", "2026-03-20") };

    let stats = compute_with(&[commute, private]);
    assert!(stats.last_activity.is_none());
  }

  #[test]
  fn test_last_activity_weight_training_sport() {
    let weight = SummaryActivity { activity_type: Some("WeightTraining".to_string()),
                                   sport_type: Some("WeightTraining".to_string()),
                                   distance: 0.0,
                                   ..make_run(0.0, 2700, "Morning Weight Training", "2026-03-25") };
    let earlier_run = make_run(10_000.0, 3000, "Long run", "2026-03-20");

    let stats = compute_with(&[earlier_run, weight]);
    let last = stats.last_activity.as_ref().unwrap();
    assert_eq!(last.name, "Morning Weight Training");
    assert_eq!(last.sport, SportType::WeightTraining);
    assert!(last.pace_or_speed.is_empty());
    assert!(!last.is_mtb);
  }

  #[test]
  fn test_format_duration_hours_under_24() {
    assert_eq!(format_duration_secs(3600.0 * 95.0 + 180.0), "3d 23h 03m");
    assert_eq!(format_duration_secs(3600.0 * 25.0), "1d 1h 00m");
    assert_eq!(format_duration_secs(3600.0 * 2.5), "2h 30m");
    assert_eq!(format_duration_secs(3600.0 * 48.0), "2d 0h 00m");
  }
}
