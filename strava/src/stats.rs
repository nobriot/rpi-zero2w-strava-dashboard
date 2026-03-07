use crate::types::{AthleteStats, SportType, SummaryActivity};

/// Per-sport YTD summary (only present for sports with ≥1 activity this year).
#[derive(Debug, Clone)]
pub struct SportSummary {
    pub sport: SportType,
    pub ytd_distance_km: f64,
    pub ytd_count: u32,
    pub ytd_time_display: String,
    pub fastest: Option<ActivityHighlight>,
    pub longest: Option<ActivityHighlight>,
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
}

/// A single activity highlighted for a specific reason (fastest, longest, last).
#[derive(Debug, Clone)]
pub struct ActivityHighlight {
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
    ) -> Self {
        let all_sport_types = [SportType::Run, SportType::Ride, SportType::Swim];

        let sports: Vec<SportSummary> = all_sport_types
            .iter()
            .filter_map(|&sport| {
                let ytd = stats.ytd_totals(sport)?;
                if ytd.count == 0 {
                    return None;
                }

                let sport_activities: Vec<&SummaryActivity> =
                    activities.iter().filter(|a| a.sport() == Some(sport)).collect();

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

                Some(SportSummary {
                    sport,
                    ytd_distance_km: ytd.distance_km(),
                    ytd_count: ytd.count,
                    ytd_time_display: format_duration_secs(ytd.moving_time),
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
        let last_activity_polyline = last_eligible
            .map(|a| a.polyline_points())
            .unwrap_or_default();

        Self {
            sports,
            last_activity,
            athlete_first_name: athlete_first_name.to_string(),
            activity_count,
            total_moving_time_secs,
            total_kudos,
            last_activity_polyline,
        }
    }

    /// Number of activities
    pub fn activity_count(&self) -> usize {
        self.activity_count
    }

    /// Format total moving time as "Xh Ym"
    pub fn total_time_display(&self) -> String {
        let hours = self.total_moving_time_secs / 3600;
        let minutes = (self.total_moving_time_secs % 3600) / 60;
        format!("{hours}h {minutes}m")
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
        println!("║              DASHBOARD STATS SUMMARY                 ║");
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

        println!("👍 Total Kudos: {}", self.total_kudos);
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
    let hours = (secs / 3600.0) as u32;
    let minutes = ((secs % 3600.0) / 60.0) as u32;
    format!("{hours}h {minutes}m")
}

fn to_highlight(a: &SummaryActivity, sport: SportType) -> ActivityHighlight {
    let pace_or_speed = match sport {
        SportType::Run => a.format_pace_per_km(),
        SportType::Ride => format!("{:.1} km/h", a.avg_speed_kmh()),
        SportType::Swim => a.format_pace_per_100m(),
    };
    ActivityHighlight {
        name: a.name.clone().unwrap_or_else(|| "Unnamed".to_string()),
        distance_km: a.distance_km(),
        moving_time_display: a.format_moving_time(),
        pace_or_speed,
        date: format_date(&a.start_date_local),
    }
}
