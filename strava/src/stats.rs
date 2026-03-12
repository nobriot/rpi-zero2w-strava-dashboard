use crate::types::{AthleteStats, SportType, SummaryActivity};

/// Per-sport YTD summary (only present for sports with ≥1 activity this year).
#[derive(Debug, Clone)]
pub struct SportSummary {
    pub sport: SportType,
    pub ytd_distance_km: f64,
    pub ytd_count: u32,
    pub ytd_time_secs: f64,
    pub ytd_time_display: String,
    pub fastest: Option<ActivityHighlight>,
    pub longest: Option<ActivityHighlight>,
}

/// Best effort for a standard running race distance.
/// Always present for each race bucket (5K, 10K, HM); fields are `None` when
/// no matching activity exists.
#[derive(Debug, Clone)]
pub struct RunRaceBest {
    pub label: &'static str,
    pub target_km: f64,
    pub distance_km: Option<f64>,
    pub moving_time_display: Option<String>,
    pub pace: Option<String>,
    pub name: Option<String>,
    pub date: Option<String>,
}

/// All the stats we want to display on the dashboard.
#[derive(Debug)]
pub struct DashboardStats {
    /// Per-sport summaries (only sports with ≥1 YTD activity)
    pub sports: Vec<SportSummary>,

    pub last_activity: Option<ActivityHighlight>,

    /// Athlete first name (for display header)
    pub athlete_first_name: String,

    /// Total number of activities (all types)
    pub activity_count: usize,
    /// Total moving time across all activities in seconds
    pub total_moving_time_secs: u32,
    /// Total kudos across all activities this year
    pub total_kudos: u32,
    /// Decoded polyline points (lat, lon) from the last activity
    pub last_activity_polyline: Vec<(f64, f64)>,

    /// Fastest run efforts at standard race distances (5K, 10K, HM)
    pub run_race_bests: Vec<RunRaceBest>,
    /// Total YTD distance across all active sports in km
    pub total_distance_km: f64,
    /// Total elevation gain across all activities in meters
    pub total_elevation_gain_m: f64,
    /// Include all sports in display even if zero activities (demo mode)
    pub show_all_sports: bool,
}

/// A single activity highlighted for a specific reason (fastest, longest, last).
#[derive(Debug, Clone)]
pub struct ActivityHighlight {
    pub sport: SportType,
    pub name: String,
    pub distance_km: f64,
    pub moving_time_display: String,
    pub pace_or_speed: String,
    pub date: String,
}

impl DashboardStats {
    /// Compute dashboard stats from aggregate athlete stats and the list of
    /// individual activities fetched for the current year.
    pub fn compute(
        stats: &AthleteStats,
        activities: &[SummaryActivity],
        athlete_first_name: &str,
        show_all_sports: bool,
    ) -> Self {
        let all_sport_types = [SportType::Run, SportType::Ride, SportType::Swim];

        let sports: Vec<SportSummary> = all_sport_types
            .iter()
            .filter_map(|&sport| {
                let ytd = stats.ytd_totals(sport);
                let count = ytd.map(|t| t.count).unwrap_or(0);
                if count == 0 && !show_all_sports {
                    return None;
                }

                let sport_activities: Vec<&SummaryActivity> = activities
                    .iter()
                    .filter(|a| a.sport() == Some(sport))
                    .collect();

                let fastest = sport_activities
                    .iter()
                    .filter(|a| a.distance > 0.0)
                    .max_by(|a, b| {
                        a.average_speed
                            .partial_cmp(&b.average_speed)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|a| to_highlight(a, sport));

                let longest = sport_activities
                    .iter()
                    .max_by(|a, b| {
                        a.distance
                            .partial_cmp(&b.distance)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|a| to_highlight(a, sport));

                let (distance_km, moving_time, time_display) = match ytd {
                    Some(t) => (
                        t.distance_km(),
                        t.moving_time,
                        format_duration_secs(t.moving_time),
                    ),
                    None => (0.0, 0.0, "0h 0m".to_string()),
                };

                Some(SportSummary {
                    sport,
                    ytd_distance_km: distance_km,
                    ytd_count: count,
                    ytd_time_secs: moving_time,
                    ytd_time_display: time_display,
                    fastest,
                    longest,
                })
            })
            .collect();

        // Last activity = first non-commute, public activity (sorted newest-first)
        let last_eligible = activities.iter().find(|a| a.can_be_displayed());
        let last_activity = last_eligible.map(|a| {
            let sport = a.sport().unwrap_or(SportType::Ride);
            to_highlight(a, sport)
        });

        let activity_count = activities.len();
        let total_moving_time_secs: u32 = activities.iter().map(|a| a.moving_time).sum();
        let total_kudos: u32 = activities.iter().map(|a| a.kudos_count).sum();
        let total_elevation_gain_m: f64 = activities.iter().map(|a| a.total_elevation_gain).sum();
        let last_activity_polyline = last_eligible
            .map(|a| a.polyline_points())
            .unwrap_or_default();

        let total_distance_km: f64 = sports.iter().map(|s| s.ytd_distance_km).sum();
        let run_race_bests = compute_race_bests(activities);

        Self {
            sports,
            last_activity,
            athlete_first_name: athlete_first_name.to_string(),
            activity_count,
            total_moving_time_secs,
            total_kudos,
            last_activity_polyline,
            run_race_bests,
            total_distance_km,
            total_elevation_gain_m,
            show_all_sports,
        }
    }

    /// Number of activities
    pub fn activity_count(&self) -> usize {
        self.activity_count
    }

    /// Format total moving time as "Xd Yh Zm" (includes days when ≥24h)
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
        self.sports
            .iter()
            .find(|s| s.sport == sport)
            .map(|s| s.ytd_distance_km)
            .unwrap_or(0.0)
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
            println!(
                "{} YTD {:?}: {:.1} km · {} activities · {}",
                icon, s.sport, s.ytd_distance_km, s.ytd_count, s.ytd_time_display
            );
            if let Some(ref a) = s.fastest {
                println!(
                    "  ⚡ Fastest: \"{}\" — {:.1} km in {} ({})",
                    a.name, a.distance_km, a.moving_time_display, a.pace_or_speed
                );
            }
            if let Some(ref a) = s.longest {
                println!(
                    "  📏 Longest: \"{}\" — {:.1} km in {}",
                    a.name, a.distance_km, a.moving_time_display
                );
            }
        }
        println!();

        if !self.run_race_bests.is_empty() {
            println!("🏁 Running Race Bests:");
            for rb in &self.run_race_bests {
                if let Some(ref pace) = rb.pace {
                    println!(
                        "  {} {} — \"{}\" ({})",
                        rb.label,
                        pace,
                        rb.name.as_deref().unwrap_or("—"),
                        rb.date.as_deref().unwrap_or("—")
                    );
                } else {
                    println!("  {} —", rb.label);
                }
            }
            println!();
        }

        println!(
            "📊 Totals: {:.1} km · {} activities · {} · {:.0}m ↑ · {} kudos",
            self.total_distance_km,
            self.activity_count,
            self.total_time_display(),
            self.total_elevation_gain_m,
            self.total_kudos
        );
        println!();

        if let Some(ref a) = self.last_activity {
            println!(
                "🕐 Last Activity: \"{}\" — {:.1} km on {}",
                a.name, a.distance_km, a.date
            );
        }
    }
}

fn format_date(raw: &Option<String>) -> String {
    raw.as_deref()
        .unwrap_or("")
        .get(..10)
        .unwrap_or("")
        .to_string()
}

fn format_duration_secs(secs: f64) -> String {
    let days = (secs / 24.0 / 3600.0) as u32;
    let hours = (secs / 3600.0) as u32;
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
    };
    ActivityHighlight {
        sport,
        name: a.name.clone().unwrap_or_else(|| "Unnamed".to_string()),
        distance_km: a.distance_km(),
        moving_time_display: a.format_moving_time(),
        pace_or_speed,
        date: format_date(&a.start_date_local),
    }
}

/// Race distance buckets for finding best running efforts.
const RACE_BUCKETS: &[(&str, f64, f64)] = &[
    // (label, target_km, min_km)
    ("5K", 5.0, 4.95),
    ("10K", 10.0, 9.9),
    ("HM", 21.1, 21.0),
];

fn compute_race_bests(activities: &[SummaryActivity]) -> Vec<RunRaceBest> {
    let runs: Vec<&SummaryActivity> = activities
        .iter()
        .filter(|a| a.is_run() && a.distance > 0.0 && a.moving_time > 0)
        .collect();

    RACE_BUCKETS
        .iter()
        .map(|&(label, target_km, min_km)| {
            let best = runs
                .iter()
                .filter(|a| a.distance_km() >= min_km)
                .min_by(|a, b| {
                    let est_a = a.moving_time as f64 * target_km / a.distance_km();
                    let est_b = b.moving_time as f64 * target_km / b.distance_km();
                    est_a
                        .partial_cmp(&est_b)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });

            match best {
                Some(b) => {
                    let estimated_secs = b.moving_time as f64 * target_km / b.distance_km();
                    let est_time = format_estimated_time(estimated_secs);
                    let pace_min = estimated_secs / 60.0 / target_km;
                    let pace_whole = pace_min.floor() as u32;
                    let pace_frac = ((pace_min - pace_whole as f64) * 60.0).round() as u32;
                    let pace = format!("{pace_whole}:{pace_frac:02} /km");

                    RunRaceBest {
                        label,
                        target_km,
                        distance_km: Some(b.distance_km()),
                        moving_time_display: Some(est_time),
                        pace: Some(pace),
                        name: Some(b.name.clone().unwrap_or_else(|| "Unnamed".to_string())),
                        date: Some(format_date(&b.start_date_local)),
                    }
                }
                None => RunRaceBest {
                    label,
                    target_km,
                    distance_km: None,
                    moving_time_display: None,
                    pace: None,
                    name: None,
                    date: None,
                },
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
        SummaryActivity {
            id: 0,
            name: Some(name.to_string()),
            activity_type: Some("Run".to_string()),
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
            kudos_count: 0,
        }
    }

    #[test]
    fn test_longer_faster_run_beats_in_bucket_run() {
        // 10.5km in 50min (in old bucket) vs 13km in 55min (outside old bucket but faster est 10K)
        let in_bucket = make_run(10_500.0, 3000, "Short run", "2026-03-08");
        let longer = make_run(13_000.0, 3300, "Interval session", "2026-03-10");

        // est 10K for in_bucket: 3000 * 10/10.5 = 2857s
        // est 10K for longer:    3300 * 10/13.0 = 2538s  ← faster
        let results = compute_race_bests(&[in_bucket, longer]);
        let ten_k = results.iter().find(|r| r.label == "10K").unwrap();

        assert_eq!(ten_k.name.as_deref(), Some("Interval session"));
    }

    #[test]
    fn test_exact_target_distance() {
        // Run exactly 10.0km
        let exact = make_run(10_000.0, 3000, "Exact 10K", "2026-03-01");
        let results = compute_race_bests(&[exact]);
        let ten_k = results.iter().find(|r| r.label == "10K").unwrap();

        assert_eq!(ten_k.name.as_deref(), Some("Exact 10K"));
        assert_eq!(ten_k.moving_time_display.as_deref(), Some("50m 00s"));
    }

    #[test]
    fn test_run_below_target_excluded() {
        // 9.0km run should NOT qualify for 10K (below min_km of 9.8)
        let short = make_run(9_000.0, 2400, "Short run", "2026-03-01");
        let results = compute_race_bests(&[short]);
        let ten_k = results.iter().find(|r| r.label == "10K").unwrap();

        assert!(
            ten_k.name.is_none(),
            "9km run should not qualify as 10K best"
        );
    }

    #[test]
    fn test_non_run_excluded() {
        // A ride should never appear in race bests
        let ride = SummaryActivity {
            activity_type: Some("Ride".to_string()),
            ..make_run(15_000.0, 2000, "Fast ride", "2026-03-01")
        };
        let results = compute_race_bests(&[ride]);
        let ten_k = results.iter().find(|r| r.label == "10K").unwrap();

        assert!(ten_k.name.is_none());
    }

    #[test]
    fn test_estimated_time_display() {
        // 12km in 60min → est 10K = 60 * 10/12 = 50min = 3000s
        let run = make_run(12_000.0, 3600, "Long run", "2026-03-01");
        let results = compute_race_bests(&[run]);
        let ten_k = results.iter().find(|r| r.label == "10K").unwrap();

        assert_eq!(ten_k.moving_time_display.as_deref(), Some("50m 00s"));
        assert_eq!(ten_k.pace.as_deref(), Some("5:00 /km"));
    }
}
