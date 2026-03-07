use crate::types::{ActivityTotal, AthleteStats, SportType, SummaryActivity};

/// All the stats we want to display on the dashboard.
#[derive(Debug)]
pub struct DashboardStats {
    pub ytd_run_distance_km: f64,
    pub ytd_ride_distance_km: f64,

    pub fastest_run: Option<ActivityHighlight>,
    pub fastest_ride: Option<ActivityHighlight>,
    pub longest_run: Option<ActivityHighlight>,
    pub longest_ride: Option<ActivityHighlight>,

    pub last_activity: Option<ActivityHighlight>,

    /// Athlete first name (for display header)
    pub athlete_first_name: String,

    /// Total number of activities
    pub activity_count: usize,
    /// Total moving time across all activities in seconds
    pub total_moving_time_secs: u32,
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
        let ytd_run_distance_km = stats
            .ytd_totals(SportType::Run)
            .map(|t: &ActivityTotal| t.distance_km())
            .unwrap_or(0.0);

        let ytd_ride_distance_km = stats
            .ytd_totals(SportType::Ride)
            .map(|t: &ActivityTotal| t.distance_km())
            .unwrap_or(0.0);

        let runs: Vec<&SummaryActivity> = activities.iter().filter(|a| a.is_run()).collect();
        let rides: Vec<&SummaryActivity> = activities.iter().filter(|a| a.is_ride()).collect();

        // Fastest run = highest average_speed among runs
        let fastest_run = runs
            .iter()
            .filter(|a| a.distance > 0.0)
            .max_by(|a, b| a.average_speed.partial_cmp(&b.average_speed).unwrap_or(std::cmp::Ordering::Equal))
            .map(|a| to_run_highlight(a));

        // Fastest ride = highest average_speed among rides
        let fastest_ride = rides
            .iter()
            .filter(|a| a.distance > 0.0)
            .max_by(|a, b| a.average_speed.partial_cmp(&b.average_speed).unwrap_or(std::cmp::Ordering::Equal))
            .map(|a| to_ride_highlight(a));

        // Longest run = max distance among runs
        let longest_run = runs
            .iter()
            .max_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Equal))
            .map(|a| to_run_highlight(a));

        // Longest ride = max distance among rides
        let longest_ride = rides
            .iter()
            .max_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Equal))
            .map(|a| to_ride_highlight(a));

        // Last activity = first in the list (API returns newest first)
        let last_activity = activities.first().map(|a| {
            if a.is_run() {
                to_run_highlight(a)
            } else {
                to_ride_highlight(a)
            }
        });

        let activity_count = activities.len();
        let total_moving_time_secs: u32 = activities.iter().map(|a| a.moving_time).sum();
        let last_activity_polyline = activities
            .first()
            .map(|a| a.polyline_points())
            .unwrap_or_default();

        Self {
            ytd_run_distance_km,
            ytd_ride_distance_km,
            fastest_run,
            fastest_ride,
            longest_run,
            longest_ride,
            last_activity,
            athlete_first_name: athlete_first_name.to_string(),
            activity_count,
            total_moving_time_secs,
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

    pub fn print_summary(&self) {
        println!("╔═══════════════════════════════════════════════════════╗");
        println!("║              DASHBOARD STATS SUMMARY                 ║");
        println!("╚═══════════════════════════════════════════════════════╝\n");

        println!("🏃 YTD Running Distance:  {:.1} km", self.ytd_run_distance_km);
        println!("🚴 YTD Cycling Distance:  {:.1} km", self.ytd_ride_distance_km);
        println!();

        if let Some(ref a) = self.fastest_run {
            println!("⚡ Fastest Run:   \"{}\" — {:.1} km in {} ({})", a.name, a.distance_km, a.moving_time_display, a.pace_or_speed);
        }
        if let Some(ref a) = self.fastest_ride {
            println!("⚡ Fastest Ride:  \"{}\" — {:.1} km in {} ({})", a.name, a.distance_km, a.moving_time_display, a.pace_or_speed);
        }
        if let Some(ref a) = self.longest_run {
            println!("📏 Longest Run:   \"{}\" — {:.1} km in {}", a.name, a.distance_km, a.moving_time_display);
        }
        if let Some(ref a) = self.longest_ride {
            println!("📏 Longest Ride:  \"{}\" — {:.1} km in {}", a.name, a.distance_km, a.moving_time_display);
        }
        println!();

        if let Some(ref a) = self.last_activity {
            println!("🕐 Last Activity: \"{}\" — {:.1} km on {}", a.name, a.distance_km, a.date);
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

fn to_run_highlight(a: &SummaryActivity) -> ActivityHighlight {
    ActivityHighlight {
        name: a.name.clone().unwrap_or_else(|| "Unnamed".to_string()),
        distance_km: a.distance_km(),
        moving_time_display: a.format_moving_time(),
        pace_or_speed: a.format_pace_per_km(),
        date: format_date(&a.start_date_local),
    }
}

fn to_ride_highlight(a: &SummaryActivity) -> ActivityHighlight {
    ActivityHighlight {
        name: a.name.clone().unwrap_or_else(|| "Unnamed".to_string()),
        distance_km: a.distance_km(),
        moving_time_display: a.format_moving_time(),
        pace_or_speed: format!("{:.1} km/h", a.avg_speed_kmh()),
        date: format_date(&a.start_date_local),
    }
}
