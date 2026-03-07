use ab_glyph::{FontRef, PxScale};
use chrono::{Datelike, Utc};
use image::{Rgb, RgbImage};
use imageproc::drawing::{draw_filled_rect_mut, draw_line_segment_mut, draw_text_mut};
use imageproc::rect::Rect;
use strava::stats::{DashboardStats, SportSummary};
use strava::types::SportType;

use crate::ina219::BatteryStatus;

const W: u32 = 800;
const H: u32 = 480;

// Colors (RGB approximations of what looks good on the 6-color EPD)
const WHITE: Rgb<u8> = Rgb([255, 255, 255]);
const BLACK: Rgb<u8> = Rgb([0, 0, 0]);
const RED: Rgb<u8> = Rgb([200, 0, 0]);
const GREEN: Rgb<u8> = Rgb([0, 128, 0]);
const BLUE: Rgb<u8> = Rgb([0, 0, 128]);

// Embedded font — Inter Regular (SIL Open Font License)
// You must place a TTF file at display/fonts/Inter-Regular.ttf
const FONT_BYTES: &[u8] = include_bytes!("../fonts/Inter-Regular.ttf");
const FONT_BOLD_BYTES: &[u8] = include_bytes!("../fonts/Inter-Bold.ttf");

/// Dashboard display configuration.
pub struct DisplayConfig {
    pub run_goal_km: f64,
    pub ride_goal_km: f64,
    pub swim_goal_km: f64,
}

impl DisplayConfig {
    fn goal_for(&self, sport: SportType) -> f64 {
        match sport {
            SportType::Run => self.run_goal_km,
            SportType::Ride => self.ride_goal_km,
            SportType::Swim => self.swim_goal_km,
        }
    }
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            run_goal_km: 2000.0,
            ride_goal_km: 5000.0,
            swim_goal_km: 200.0,
        }
    }
}

fn sport_accent(sport: SportType) -> Rgb<u8> {
    match sport {
        SportType::Run => RED,
        SportType::Ride => BLUE,
        SportType::Swim => GREEN,
    }
}

fn sport_label(sport: SportType) -> &'static str {
    match sport {
        SportType::Run => "RUN",
        SportType::Ride => "RIDE",
        SportType::Swim => "SWIM",
    }
}

/// Render the full dashboard as an 800×480 RGB image.
pub fn render_dashboard(
    stats: &DashboardStats,
    battery: Option<&BatteryStatus>,
    config: &DisplayConfig,
    avatar: Option<&[u8]>,
) -> RgbImage {
    let mut img = RgbImage::from_pixel(W, H, WHITE);

    let font = FontRef::try_from_slice(FONT_BYTES).expect("Failed to load font");
    let font_bold = FontRef::try_from_slice(FONT_BOLD_BYTES).expect("Failed to load bold font");

    draw_header(&mut img, &font_bold, stats, battery, avatar);
    let y = draw_sport_bars(&mut img, &font, &font_bold, stats, config);
    let stats_y = draw_stats_row(&mut img, &font, &font_bold, stats, y);
    draw_latest_activity(&mut img, &font, &font_bold, stats, stats_y);
    draw_polyline(&mut img, stats, y);

    img
}

fn draw_header(
    img: &mut RgbImage,
    font_bold: &FontRef,
    stats: &DashboardStats,
    battery: Option<&BatteryStatus>,
    avatar: Option<&[u8]>,
) {
    let header_h = 60;
    draw_filled_rect_mut(img, Rect::at(0, 0).of_size(W, header_h), RED);

    // Avatar on the left
    if let Some(bytes) = avatar {
        draw_avatar(img, bytes);
    }

    let year = Utc::now().year();
    let title = format!("STRAVA  |  {} - {}", stats.athlete_first_name, year);
    draw_text_mut(
        img,
        WHITE,
        center_x_text(W, &title, 28),
        12,
        PxScale::from(28.0),
        font_bold,
        &title,
    );

    // Battery indicator
    if let Some(bat) = battery {
        let bat_text = format!("{}%", bat.percentage);
        draw_text_mut(
            img,
            WHITE,
            (W - 80) as i32,
            18,
            PxScale::from(20.0),
            font_bold,
            &bat_text,
        );
    }
}

const AVATAR_SIZE: u32 = 44;
const AVATAR_PAD: i64 = 8;

fn draw_avatar(img: &mut RgbImage, avatar_bytes: &[u8]) {
    let avatar = match image::load_from_memory(avatar_bytes) {
        Ok(a) => a,
        Err(e) => {
            log::warn!("Failed to decode avatar image: {e}");
            return;
        }
    };
    let resized = avatar.resize_exact(
        AVATAR_SIZE,
        AVATAR_SIZE,
        image::imageops::FilterType::Triangle,
    );
    let rgb = resized.to_rgb8();
    image::imageops::overlay(img, &rgb, AVATAR_PAD, AVATAR_PAD);
}

/// Draw one goal bar per active sport. Returns the Y position after the last bar.
fn draw_sport_bars(
    img: &mut RgbImage,
    font: &FontRef,
    font_bold: &FontRef,
    stats: &DashboardStats,
    config: &DisplayConfig,
) -> i32 {
    let margin = 24i32;
    let bar_w = (W as i32 - 2 * margin) as u32;
    let bar_h = 22u32;
    let section_h = 58i32;
    let mut y = 68i32;

    for summary in &stats.sports {
        let accent = sport_accent(summary.sport);
        let goal = config.goal_for(summary.sport);
        let label = sport_label(summary.sport);

        draw_single_bar(
            img, font, font_bold, label, summary, goal, margin, y, bar_w, bar_h, accent,
        );

        y += section_h;
    }

    y + 6
}

fn draw_single_bar(
    img: &mut RgbImage,
    font: &FontRef,
    font_bold: &FontRef,
    label: &str,
    summary: &SportSummary,
    goal_km: f64,
    margin: i32,
    y_start: i32,
    bar_w: u32,
    bar_h: u32,
    accent: Rgb<u8>,
) {
    // Label + distance on the same line
    let value_text = format!(
        "{}   {:.0} / {:.0} km",
        label, summary.ytd_distance_km, goal_km
    );
    draw_text_mut(
        img,
        accent,
        margin,
        y_start,
        PxScale::from(18.0),
        font_bold,
        &value_text,
    );

    // Progress bar
    let bar_y = (y_start + 22) as u32;

    // Border
    draw_filled_rect_mut(
        img,
        Rect::at(margin, bar_y as i32).of_size(bar_w, bar_h),
        BLACK,
    );
    draw_filled_rect_mut(
        img,
        Rect::at(margin + 2, bar_y as i32 + 2).of_size(bar_w - 4, bar_h - 4),
        WHITE,
    );

    // Fill (uses sport accent color)
    let pct = if goal_km > 0.0 {
        (summary.ytd_distance_km / goal_km).min(1.0)
    } else {
        0.0
    };
    let fill_w = ((bar_w - 4) as f64 * pct) as u32;
    if fill_w > 0 {
        draw_filled_rect_mut(
            img,
            Rect::at(margin + 2, bar_y as i32 + 2).of_size(fill_w, bar_h - 4),
            accent,
        );
    }

    // Sub-text: percentage/progress + per-sport totals
    let progress = if summary.ytd_distance_km >= goal_km {
        format!("+{:.0} km above goal", summary.ytd_distance_km - goal_km)
    } else {
        format!(
            "{:.1}%  ·  {:.0} km to go",
            pct * 100.0,
            goal_km - summary.ytd_distance_km
        )
    };
    let sub_text = format!(
        "{}  ·  {} activities  ·  {}",
        progress, summary.ytd_count, summary.ytd_time_display
    );
    draw_text_mut(
        img,
        BLACK,
        margin,
        (bar_y + bar_h + 4) as i32,
        PxScale::from(14.0),
        font,
        &sub_text,
    );
}

/// Draw the stats row (activities, kudos, last route label). Returns Y for next section.
fn draw_stats_row(
    img: &mut RgbImage,
    font: &FontRef,
    font_bold: &FontRef,
    stats: &DashboardStats,
    y_start: i32,
) -> i32 {
    let margin = 24i32;
    let col_w = (W as i32 - 2 * margin) / 3;

    // Column 1: Activities count
    draw_text_mut(
        img,
        RED,
        margin,
        y_start,
        PxScale::from(20.0),
        font_bold,
        "ACTIVITIES",
    );
    draw_text_mut(
        img,
        BLACK,
        margin,
        y_start + 28,
        PxScale::from(22.0),
        font,
        &stats.activity_count().to_string(),
    );

    // Column 2: Kudos
    let col2_x = margin + col_w;
    draw_text_mut(
        img,
        RED,
        col2_x,
        y_start,
        PxScale::from(20.0),
        font_bold,
        "KUDOS",
    );
    draw_text_mut(
        img,
        BLACK,
        col2_x,
        y_start + 28,
        PxScale::from(22.0),
        font,
        &stats.total_kudos.to_string(),
    );

    // Column 3: Last Route label (polyline drawn separately)
    let col3_x = margin + 2 * col_w;
    draw_text_mut(
        img,
        RED,
        col3_x,
        y_start,
        PxScale::from(20.0),
        font_bold,
        "LAST ROUTE",
    );

    y_start + 60
}

fn draw_latest_activity(
    img: &mut RgbImage,
    font: &FontRef,
    font_bold: &FontRef,
    stats: &DashboardStats,
    y_start: i32,
) {
    let margin = 24i32;

    if let Some(ref last) = stats.last_activity {
        draw_text_mut(
            img,
            RED,
            margin,
            y_start,
            PxScale::from(20.0),
            font_bold,
            "LATEST ACTIVITY",
        );

        // Date
        draw_text_mut(
            img,
            BLACK,
            margin,
            y_start + 28,
            PxScale::from(18.0),
            font,
            &last.date,
        );

        // Column headers
        let header_y = y_start + 58;
        let val_y = y_start + 80;
        let col_w = 170i32;

        draw_text_mut(
            img,
            BLACK,
            margin,
            header_y,
            PxScale::from(14.0),
            font_bold,
            "Distance",
        );
        draw_text_mut(
            img,
            BLACK,
            margin + col_w,
            header_y,
            PxScale::from(14.0),
            font_bold,
            "Pace / Speed",
        );
        draw_text_mut(
            img,
            BLACK,
            margin + 2 * col_w,
            header_y,
            PxScale::from(14.0),
            font_bold,
            "Time",
        );

        // Values
        draw_text_mut(
            img,
            BLACK,
            margin,
            val_y,
            PxScale::from(18.0),
            font,
            &format!("{:.2} km", last.distance_km),
        );
        draw_text_mut(
            img,
            BLACK,
            margin + col_w,
            val_y,
            PxScale::from(18.0),
            font,
            &last.pace_or_speed,
        );
        draw_text_mut(
            img,
            BLACK,
            margin + 2 * col_w,
            val_y,
            PxScale::from(18.0),
            font,
            &last.moving_time_display,
        );

        // Activity name
        draw_text_mut(
            img,
            BLUE,
            margin,
            y_start + 110,
            PxScale::from(16.0),
            font,
            &last.name,
        );
    }
}

fn draw_polyline(img: &mut RgbImage, stats: &DashboardStats, poly_y_start: i32) {
    let points = &stats.last_activity_polyline;
    if points.is_empty() {
        return;
    }

    // Draw area: right third column, below stats row header
    let margin = 24u32;
    let col_w = (W - 2 * margin) / 3;
    let area_x = margin + 2 * col_w;
    let area_y = (poly_y_start + 28) as u32;
    let area_w = col_w - 10;
    let area_h = H.saturating_sub(area_y + 20);

    if area_h < 20 {
        return;
    }

    // Find bounds
    let (mut min_lat, mut max_lat) = (f64::MAX, f64::MIN);
    let (mut min_lon, mut max_lon) = (f64::MAX, f64::MIN);
    for &(lat, lon) in points {
        min_lat = min_lat.min(lat);
        max_lat = max_lat.max(lat);
        min_lon = min_lon.min(lon);
        max_lon = max_lon.max(lon);
    }

    let lat_range = (max_lat - min_lat).max(1e-6);
    let lon_range = (max_lon - min_lon).max(1e-6);

    // Fit to area preserving aspect ratio
    let route_aspect = lon_range / lat_range;
    let area_aspect = area_w as f64 / area_h as f64;

    let (draw_w, draw_h, off_x, off_y) = if route_aspect > area_aspect {
        let dw = area_w as f64;
        let dh = dw / route_aspect;
        (dw, dh, 0.0, (area_h as f64 - dh) / 2.0)
    } else {
        let dh = area_h as f64;
        let dw = dh * route_aspect;
        (dw, dh, (area_w as f64 - dw) / 2.0, 0.0)
    };

    // Draw route segments
    for window in points.windows(2) {
        let (lat0, lon0) = window[0];
        let (lat1, lon1) = window[1];

        let x0 = area_x as f32 + off_x as f32 + ((lon0 - min_lon) / lon_range * draw_w) as f32;
        let y0 =
            area_y as f32 + off_y as f32 + ((1.0 - (lat0 - min_lat) / lat_range) * draw_h) as f32;
        let x1 = area_x as f32 + off_x as f32 + ((lon1 - min_lon) / lon_range * draw_w) as f32;
        let y1 =
            area_y as f32 + off_y as f32 + ((1.0 - (lat1 - min_lat) / lat_range) * draw_h) as f32;

        draw_line_segment_mut(img, (x0, y0), (x1, y1), BLACK);
        // Draw slightly thicker by offsetting
        draw_line_segment_mut(img, (x0 + 1.0, y0), (x1 + 1.0, y1), BLACK);
    }
}

/// Estimate the x position to roughly center text of a given length.
fn center_x_text(img_width: u32, text: &str, font_size: u32) -> i32 {
    let approx_char_w = font_size as f32 * 0.55;
    let text_w = text.len() as f32 * approx_char_w;
    ((img_width as f32 - text_w) / 2.0) as i32
}
