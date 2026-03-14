use serde::Deserialize;
use serde::Serialize;

use super::athlete_stats::SportType;

/// Summary activity from Strava API
/// Endpoint: GET /athlete/activities
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SummaryActivity {
    /// The unique identifier of the activity
    pub id: u64,

    /// The name of the activity
    #[serde(default)]
    pub name: Option<String>,

    /// The activity type (Run, Ride, Swim, etc.) — deprecated by Strava
    #[serde(rename = "type", default)]
    pub activity_type: Option<String>,

    /// The sport type (fine-grained: TrailRun, MountainBikeRide, etc.)
    #[serde(default)]
    pub sport_type: Option<String>,

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

    /// Whether this activity is flagged as a commute
    #[serde(default)]
    pub commute: bool,

    /// Whether this activity is marked as private
    #[serde(default)]
    pub private: bool,

    /// The number of kudos received
    #[serde(default)]
    pub kudos_count: u32,
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
        self.sport() == Some(SportType::Run)
    }

    pub fn is_ride(&self) -> bool {
        self.sport() == Some(SportType::Ride)
    }

    pub fn is_swim(&self) -> bool {
        self.sport() == Some(SportType::Swim)
    }

    /// Map `sport_type` string to a [`SportType`] enum value.
    ///
    /// Prefers the newer `sport_type` field; falls back to the deprecated `type`
    /// field so that cached data without `sport_type` still classifies correctly.
    /// Includes virtual activities, excludes electric-assisted.
    pub fn sport(&self) -> Option<SportType> {
        // Try sport_type first (fine-grained, current API field)
        if let Some(st) = self.sport_type.as_deref() {
            return match st {
                "Run" | "TrailRun" | "VirtualRun" => Some(SportType::Run),
                "Ride" | "MountainBikeRide" | "GravelRide" | "VirtualRide" | "Handcycle"
                | "Velomobile" => Some(SportType::Ride),
                "Swim" => Some(SportType::Swim),
                // Explicitly exclude electric-assisted and unknown types
                _ => None,
            };
        }
        // Fallback to deprecated type field (for cached data without sport_type)
        match self.activity_type.as_deref() {
            Some("Run") => Some(SportType::Run),
            Some("Ride") => Some(SportType::Ride),
            Some("Swim") => Some(SportType::Swim),
            _ => None,
        }
    }

    /// Format pace as "M:SS /100m" (for swimming)
    pub fn format_pace_per_100m(&self) -> String {
        if self.distance > 0.0 {
            let pace_secs = self.moving_time as f64 / (self.distance / 100.0);
            let minutes = (pace_secs / 60.0).floor() as u32;
            let seconds = (pace_secs % 60.0).round() as u32;
            format!("{minutes}:{seconds:02} /100m")
        } else {
            "--:-- /100m".to_string()
        }
    }

    /// Whether this activity is public and not a commute.
    pub fn can_be_displayed(&self) -> bool {
        !self.commute && !self.private
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
        let (dlat, next) = decode_value(bytes, i);
        i = next;
        lat += dlat;

        if i >= bytes.len() {
            break;
        }

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

    if result & 1 != 0 {
        result = !(result >> 1);
    } else {
        result >>= 1;
    }

    (result, i)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_polyline() {
        // Standard Google example: encodes (38.5, -120.2), (40.7, -120.95), (43.252, -126.453)
        let points = decode_polyline("_p~iF~ps|U_ulLnnqC_mqNvxq`@");
        assert_eq!(points.len(), 3);
        assert!((points[0].0 - 38.5).abs() < 0.001);
        assert!((points[0].1 - (-120.2)).abs() < 0.001);
        assert!((points[2].0 - 43.252).abs() < 0.001);
    }

    #[test]
    fn test_summary_activity_with_polyline() {
        let json = r#"{
            "id": 123,
            "name": "Morning Run",
            "type": "Run",
            "sport_type": "Run",
            "distance": 10000.0,
            "moving_time": 3000,
            "elapsed_time": 3200,
            "total_elevation_gain": 50.0,
            "average_speed": 3.33,
            "max_speed": 4.0,
            "start_date_local": "2026-03-06T08:00:00Z",
            "map": {
                "summary_polyline": "_p~iF~ps|U_ulLnnqC_mqNvxq`@"
            }
        }"#;
        let activity: SummaryActivity = serde_json::from_str(json).unwrap();
        let points = activity.polyline_points();
        assert_eq!(points.len(), 3);
        assert!(activity.map.is_some());
        assert_eq!(activity.sport(), Some(SportType::Run));
    }

    #[test]
    fn test_sport_type_run_variants() {
        for sport_type in ["Run", "TrailRun", "VirtualRun"] {
            let json = format!(
                r#"{{"id":1,"sport_type":"{}","distance":5000.0}}"#,
                sport_type
            );
            let a: SummaryActivity = serde_json::from_str(&json).unwrap();
            assert_eq!(
                a.sport(),
                Some(SportType::Run),
                "{sport_type} should map to Run"
            );
            assert!(a.is_run(), "{sport_type} should be is_run()");
        }
    }

    #[test]
    fn test_sport_type_ride_variants() {
        for sport_type in [
            "Ride",
            "MountainBikeRide",
            "GravelRide",
            "VirtualRide",
            "Handcycle",
            "Velomobile",
        ] {
            let json = format!(
                r#"{{"id":1,"sport_type":"{}","distance":20000.0}}"#,
                sport_type
            );
            let a: SummaryActivity = serde_json::from_str(&json).unwrap();
            assert_eq!(
                a.sport(),
                Some(SportType::Ride),
                "{sport_type} should map to Ride"
            );
            assert!(a.is_ride(), "{sport_type} should be is_ride()");
        }
    }

    #[test]
    fn test_sport_type_swim() {
        let json = r#"{"id":1,"sport_type":"Swim","distance":1500.0}"#;
        let a: SummaryActivity = serde_json::from_str(&json).unwrap();
        assert_eq!(a.sport(), Some(SportType::Swim));
        assert!(a.is_swim());
    }

    #[test]
    fn test_sport_type_ebike_excluded() {
        for sport_type in ["EBikeRide", "EMountainBikeRide"] {
            let json = format!(
                r#"{{"id":1,"sport_type":"{}","distance":30000.0}}"#,
                sport_type
            );
            let a: SummaryActivity = serde_json::from_str(&json).unwrap();
            assert_eq!(a.sport(), None, "{sport_type} should be excluded");
            assert!(!a.is_ride(), "{sport_type} should NOT be is_ride()");
        }
    }

    #[test]
    fn test_sport_type_unknown_excluded() {
        let json = r#"{"id":1,"sport_type":"Yoga","distance":0.0}"#;
        let a: SummaryActivity = serde_json::from_str(&json).unwrap();
        assert_eq!(a.sport(), None);
    }

    #[test]
    fn test_missing_sport_type_falls_back_to_type() {
        let json = r#"{"id":1,"type":"Run","distance":5000.0}"#;
        let a: SummaryActivity = serde_json::from_str(&json).unwrap();
        assert_eq!(
            a.sport(),
            Some(SportType::Run),
            "should fall back to deprecated type field"
        );
    }

    #[test]
    fn test_missing_both_fields() {
        let json = r#"{"id":1,"distance":5000.0}"#;
        let a: SummaryActivity = serde_json::from_str(&json).unwrap();
        assert_eq!(a.sport(), None);
    }
}
