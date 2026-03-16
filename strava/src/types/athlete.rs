use serde::{Deserialize, Serialize};

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

  /// URL to a 124×124 pixel profile picture
  #[serde(default)]
  pub profile: Option<String>,

  /// URL to a 62×62 pixel profile picture
  #[serde(default)]
  pub profile_medium: Option<String>,

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
    let parts: Vec<String> =
      [self.city.as_ref(), self.country.as_ref()].iter()
                                                 .filter_map(|&opt| opt.cloned())
                                                 .collect();

    if parts.is_empty() { None } else { Some(parts.join(", ")) }
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

#[cfg(test)]
mod tests {
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
    let mut athlete = DetailedAthlete { id:                     1,
                                        firstname:              Some("Jane".to_string()),
                                        lastname:               Some("Smith".to_string()),
                                        city:                   None,
                                        country:                None,
                                        premium:                false,
                                        summit:                 false,
                                        profile:                None,
                                        profile_medium:         None,
                                        follower_count:         None,
                                        friend_count:           None,
                                        measurement_preference: None,
                                        bikes:                  vec![],
                                        shoes:                  vec![], };

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
