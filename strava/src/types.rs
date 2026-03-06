use serde::Deserialize;
use serde::Serialize;

/// Expected response body for the Access Token endpoint
#[derive(Debug, Deserialize, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
}

/// Detailed athlete information from Strava API
/// Endpoint: GET /athlete
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DetailedAthlete {
    /// The unique identifier of the athlete
    pub id: u64,

    /// The athlete's first name
    #[serde(default)]
    pub firstname: Option<String>,

    /// The athlete's last name
    #[serde(default)]
    pub lastname: Option<String>,

    /// The athlete's city
    #[serde(default)]
    pub city: Option<String>,

    /// The athlete's country
    #[serde(default)]
    pub country: Option<String>,

    /// Whether the athlete is a premium (Summit) member
    #[serde(default)]
    pub premium: bool,

    /// Whether the athlete is a Strava Summit member
    #[serde(default)]
    pub summit: bool,

    /// The athlete's follower count
    #[serde(default)]
    pub follower_count: Option<u32>,

    /// The athlete's friend count
    #[serde(default)]
    pub friend_count: Option<u32>,

    /// The athlete's preferred unit system (feet or meters)
    #[serde(default)]
    pub measurement_preference: Option<MeasurementPreference>,

    /// Bikes owned by the athlete
    #[serde(default)]
    pub bikes: Vec<SummaryGear>,

    /// Shoes owned by the athlete
    #[serde(default)]
    pub shoes: Vec<SummaryGear>,
}

/// Measurement preference (feet or meters)
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MeasurementPreference {
    Feet,
    Meters,
}

/// Summary club information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SummaryClub {
    /// The club's unique identifier
    pub id: u64,

    /// Resource state
    pub resource_state: u8,

    /// The club's name
    pub name: String,

    /// URL to a 62x62 pixel profile picture
    #[serde(default)]
    pub profile_medium: Option<String>,

    /// URL to a 124x124 pixel profile picture
    #[serde(default)]
    pub profile: Option<String>,

    /// The club's vanity URL
    #[serde(default)]
    pub cover_photo: Option<String>,

    /// The club's cover photo URL
    #[serde(default)]
    pub cover_photo_small: Option<String>,

    /// The club's sport type
    #[serde(default)]
    pub sport_type: Option<String>,

    /// The club's city
    #[serde(default)]
    pub city: Option<String>,

    /// The club's state or geographical region
    #[serde(default)]
    pub state: Option<String>,

    /// The club's country
    #[serde(default)]
    pub country: Option<String>,

    /// Whether the club is private
    #[serde(rename = "private")]
    pub is_private: bool,

    /// The club's member count
    pub member_count: u32,

    /// Whether the club is featured
    pub featured: bool,

    /// Whether the club is verified
    pub verified: bool,

    /// The club's URL
    pub url: String,
}

/// Summary gear (bike or shoe) information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SummaryGear {
    /// The gear's unique identifier
    pub id: String,

    /// Resource state
    pub resource_state: u8,

    /// Whether this is the athlete's default bike/shoe
    pub primary: bool,

    /// The gear's name
    pub name: String,

    /// The gear's distance in meters
    pub distance: f64,
}

impl DetailedAthlete {
    /// Get the athlete's full name
    pub fn full_name(&self) -> String {
        match (&self.firstname, &self.lastname) {
            (Some(first), Some(last)) => format!("{} {}", first, last),
            (Some(first), None) => first.clone(),
            (None, Some(last)) => last.clone(),
            (None, None) => "Unknown Athlete".to_string(),
        }
    }

    /// Get the athlete's location as a formatted string
    pub fn location(&self) -> Option<String> {
        let parts: Vec<String> = [self.city.as_ref(), self.country.as_ref()]
            .iter()
            .filter_map(|&opt| opt.cloned())
            .collect();

        if parts.is_empty() {
            None
        } else {
            Some(parts.join(", "))
        }
    }

    /// Check if athlete is a premium member
    pub fn is_premium(&self) -> bool {
        self.premium || self.summit
    }

    /// Get total distance across all bikes in km
    pub fn total_bike_distance_km(&self) -> f64 {
        self.bikes.iter().map(|bike| bike.distance / 1000.0).sum()
    }

    /// Get total distance across all shoes in km
    pub fn total_shoe_distance_km(&self) -> f64 {
        self.shoes.iter().map(|shoe| shoe.distance / 1000.0).sum()
    }
}

/// Athlete statistics from Strava API
/// Endpoint: GET /athletes/{id}/stats
#[derive(Debug, Clone, Deserialize, Serialize)]
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

// {"biggest_ride_distance":105024.0,"biggest_climb_elevation_gain":866.4000000000001,"recent_ride_totals":{"count":19,"distance":235382.9,"moving_time":49536,"elapsed_time":366630,"elevation_gain":672.0,"achievement_count":0},"all_ride_totals":{"count":1092,"distance":26477853.399999976,"moving_time":5224068,"elapsed_time":16601712,"elevation_gain":245995.29999999996},"recent_run_totals":{"count":13,"distance":123609.3,"moving_time":36975,"elapsed_time":37372,"elevation_gain":568.0,"achievement_count":0},"all_run_totals":{"count":2306,"distance":20890758.500000004,"moving_time":6015006,"elapsed_time":7018778,"elevation_gain":124145.89999999998},"recent_swim_totals":{"count":0,"distance":0,"moving_time":0,"elapsed_time":0,"elevation_gain":0,"achievement_count":0},"all_swim_totals":{"count":27,"distance":37567.2,"moving_time":50693,"elapsed_time":62760,"elevation_gain":0.0},"ytd_ride_totals":{"count":63,"distance":833695,"moving_time":170789.0,"elapsed_time":1169230.0,"elevation_gain":2089.0},"ytd_run_totals":{"count":25,"distance":212996,"moving_time":63140.0,"elapsed_time":63598.0,"elevation_gain":1110.0},"ytd_swim_totals":{"count":0,"distance":0,"moving_time":0,"elapsed_time":0,"elevation_gain":0}}

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
            } else if matches!(sport, SportType::Ride) && let Some(speed) = ytd.avg_speed_kmh() {
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

/// Summary activity from Strava API
/// Endpoint: GET /athlete/activities
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SummaryActivity {
    /// The unique identifier of the activity
    pub id: u64,

    /// The name of the activity
    #[serde(default)]
    pub name: Option<String>,

    /// The activity type (Run, Ride, Swim, etc.)
    #[serde(rename = "type", default)]
    pub activity_type: Option<String>,

    /// The activity's distance in meters
    #[serde(default)]
    pub distance: f64,

    /// The activity's moving time in seconds
    #[serde(default)]
    pub moving_time: u32,

    /// The activity's elapsed time in seconds
    #[serde(default)]
    pub elapsed_time: u32,

    /// The activity's total elevation gain in meters
    #[serde(default)]
    pub total_elevation_gain: f64,

    /// The activity's average speed in meters per second
    #[serde(default)]
    pub average_speed: f64,

    /// The activity's max speed in meters per second
    #[serde(default)]
    pub max_speed: f64,

    /// The start date of the activity (ISO 8601)
    #[serde(default)]
    pub start_date: Option<String>,

    /// The start date of the activity in local timezone (ISO 8601)
    #[serde(default)]
    pub start_date_local: Option<String>,

    /// Map with summary polyline for route visualization
    #[serde(default)]
    pub map: Option<PolylineMap>,
}

/// Map data from a Strava activity, containing an encoded polyline.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PolylineMap {
    /// Google-encoded polyline of the activity route
    #[serde(default)]
    pub summary_polyline: Option<String>,
}

impl SummaryActivity {
    /// Distance in kilometers
    pub fn distance_km(&self) -> f64 {
        self.distance / 1000.0
    }

    /// Average speed in km/h
    pub fn avg_speed_kmh(&self) -> f64 {
        self.average_speed * 3.6
    }

    /// Average pace in min/km (for running)
    pub fn avg_pace_min_per_km(&self) -> Option<f64> {
        if self.distance > 0.0 {
            Some((self.moving_time as f64 / 60.0) / (self.distance / 1000.0))
        } else {
            None
        }
    }

    /// Format pace as "M:SS /km"
    pub fn format_pace_per_km(&self) -> String {
        if let Some(pace) = self.avg_pace_min_per_km() {
            let minutes = pace.floor() as u32;
            let seconds = ((pace - minutes as f64) * 60.0).round() as u32;
            format!("{minutes}:{seconds:02} /km")
        } else {
            "--:-- /km".to_string()
        }
    }

    /// Format moving time as "Xh Ym Zs"
    pub fn format_moving_time(&self) -> String {
        let hours = self.moving_time / 3600;
        let minutes = (self.moving_time % 3600) / 60;
        let seconds = self.moving_time % 60;

        if hours > 0 {
            format!("{hours}h {minutes}m {seconds}s")
        } else if minutes > 0 {
            format!("{minutes}m {seconds}s")
        } else {
            format!("{seconds}s")
        }
    }

    pub fn is_run(&self) -> bool {
        self.activity_type.as_deref() == Some("Run")
    }

    pub fn is_ride(&self) -> bool {
        self.activity_type.as_deref() == Some("Ride")
    }

    /// Get the decoded polyline points as (lat, lon) pairs.
    pub fn polyline_points(&self) -> Vec<(f64, f64)> {
        self.map
            .as_ref()
            .and_then(|m| m.summary_polyline.as_deref())
            .map(decode_polyline)
            .unwrap_or_default()
    }
}

/// Decode a Google Encoded Polyline into a list of (latitude, longitude) pairs.
/// See: https://developers.google.com/maps/documentation/utilities/polylinealgorithm
pub fn decode_polyline(encoded: &str) -> Vec<(f64, f64)> {
    let mut points = Vec::new();
    let mut lat: i64 = 0;
    let mut lng: i64 = 0;
    let bytes = encoded.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        // Decode latitude delta
        let (dlat, next) = decode_value(bytes, i);
        i = next;
        lat += dlat;

        if i >= bytes.len() {
            break;
        }

        // Decode longitude delta
        let (dlng, next) = decode_value(bytes, i);
        i = next;
        lng += dlng;

        points.push((lat as f64 / 1e5, lng as f64 / 1e5));
    }

    points
}

fn decode_value(bytes: &[u8], start: usize) -> (i64, usize) {
    let mut result: i64 = 0;
    let mut shift = 0;
    let mut i = start;

    loop {
        if i >= bytes.len() {
            break;
        }
        let b = (bytes[i] as i64) - 63;
        i += 1;
        result |= (b & 0x1F) << shift;
        shift += 5;
        if b < 0x20 {
            break;
        }
    }

    // Undo the two's complement encoding
    if result & 1 != 0 {
        result = !(result >> 1);
    } else {
        result >>= 1;
    }

    (result, i)
}

/// Sport type enum for querying stats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

        // Test recent run totals
        let recent_run = stats.recent_totals(SportType::Run).unwrap();
        assert_eq!(recent_run.count, 10);
        assert_eq!(recent_run.distance_km(), 50.0);
        assert_eq!(recent_run.moving_time_hours(), 4.0);

        // Test YTD ride totals
        let ytd_ride = stats.ytd_totals(SportType::Ride).unwrap();
        assert_eq!(ytd_ride.count, 50);
        assert_eq!(ytd_ride.distance_km(), 500.0);

        // Test all time run totals
        let all_run = stats.all_time_totals(SportType::Run).unwrap();
        assert_eq!(all_run.count, 500);
        assert_eq!(all_run.distance_km(), 2500.0);
    }

    #[test]
    fn test_activity_total_conversions() {
        let total = ActivityTotal {
            count: 10,
            distance: 10000.0,   // 10 km
            moving_time: 3600.0, // 1 hour
            elapsed_time: 3700.0,
            elevation_gain: 100.0,
            achievement_count: Some(5),
        };

        assert_eq!(total.distance_km(), 10.0);
        assert!((total.distance_miles() - 6.2137).abs() < 0.01);
        assert_eq!(total.moving_time_hours(), 1.0);
        assert!((total.elevation_gain_feet() - 328.08).abs() < 0.1);

        // Test pace calculations
        let pace_km = total.avg_pace_min_per_km().unwrap();
        assert!((pace_km - 6.0).abs() < 0.01); // 6 min/km

        let speed_kmh = total.avg_speed_kmh().unwrap();
        assert!((speed_kmh - 10.0).abs() < 0.01); // 10 km/h
    }

    #[test]
    fn test_format_functions() {
        let total = ActivityTotal {
            count: 1,
            distance: 5000.0,    // 5 km
            moving_time: 1500.0, // 25 minutes
            elapsed_time: 1600.0,
            elevation_gain: 50.0,
            achievement_count: None,
        };

        assert_eq!(total.format_moving_time(), "25m 0s");
        assert_eq!(total.format_pace_per_km(), "5:00 /km");

        let long_total = ActivityTotal {
            count: 1,
            distance: 100000.0,
            moving_time: 14523.0, // 4h 2m 3s
            elapsed_time: 15000.0,
            elevation_gain: 1000.0,
            achievement_count: None,
        };

        assert_eq!(long_total.format_moving_time(), "4h 2m 3s");
    }

    #[test]
    fn test_zero_distance() {
        let total = ActivityTotal {
            count: 0,
            distance: 0.0,
            moving_time: 0.0,
            elapsed_time: 0.0,
            elevation_gain: 0.0,
            achievement_count: None,
        };

        assert!(total.avg_pace_min_per_km().is_none());
        assert!(total.avg_speed_kmh().is_none());
        assert_eq!(total.format_pace_per_km(), "--:-- /km");
    }
}

#[cfg(test)]
mod athlete_tests {
    use super::*;

    #[test]
    fn test_deserialize_athlete() {
        let json = r#"{
            "id": 12345678,
            "resource_state": 3,
            "firstname": "Nicolas",
            "lastname": "Woltmann",
            "profile_medium": "https://example.com/profile_medium.jpg",
            "profile": "https://example.com/profile.jpg",
            "city": "Copenhagen",
            "state": "Sjælland",
            "country": "Denmark",
            "sex": "M",
            "premium": true,
            "summit": true,
            "created_at": "2015-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z",
            "follower_count": 100,
            "friend_count": 50,
            "measurement_preference": "meters",
            "ftp": 250,
            "weight": 70.0,
            "clubs": [],
            "bikes": [
                {
                    "id": "b12345",
                    "resource_state": 2,
                    "primary": true,
                    "name": "My Road Bike",
                    "distance": 5000000.0
                }
            ],
            "shoes": []
        }"#;

        let athlete: DetailedAthlete = serde_json::from_str(json).unwrap();

        assert_eq!(athlete.id, 12345678);
        assert_eq!(athlete.full_name(), "Nicolas Woltmann");
        assert_eq!(athlete.location(), Some("Copenhagen, Denmark".to_string()));
        assert!(athlete.is_premium());
        assert_eq!(athlete.bikes.len(), 1);
        assert_eq!(athlete.total_bike_distance_km(), 5000.0);
    }

    #[test]
    fn test_full_name() {
        let mut athlete = DetailedAthlete {
            id: 1,
            firstname: Some("Jane".to_string()),
            lastname: Some("Smith".to_string()),
            city: None,
            country: None,
            premium: false,
            summit: false,
            follower_count: None,
            friend_count: None,
            measurement_preference: None,
            bikes: vec![],
            shoes: vec![],
        };

        assert_eq!(athlete.full_name(), "Jane Smith");

        athlete.lastname = None;
        assert_eq!(athlete.full_name(), "Jane");

        athlete.firstname = None;
        athlete.lastname = Some("Smith".to_string());
        assert_eq!(athlete.full_name(), "Smith");

        athlete.lastname = None;
        assert_eq!(athlete.full_name(), "Unknown Athlete");
    }
}
