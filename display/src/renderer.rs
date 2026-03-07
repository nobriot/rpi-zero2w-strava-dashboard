use ab_glyph::{FontRef, PxScale};
use chrono::{Datelike, Utc};
use image::{Rgb, RgbImage};
use imageproc::drawing::{draw_filled_rect_mut, draw_line_segment_mut, draw_text_mut};
use imageproc::rect::Rect;
use strava::stats::DashboardStats;

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
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            run_goal_km: 2000.0,
            ride_goal_km: 5000.0,
        }
    }
}

/// Render the full dashboard as an 800×480 RGB image.
pub fn render_dashboard(
    stats: &DashboardStats,
    battery: Option<&BatteryStatus>,
    config: &DisplayConfig,
) -> RgbImage {
    let mut img = RgbImage::from_pixel(W, H, WHITE);

    let font = FontRef::try_from_slice(FONT_BYTES).expect("Failed to load font");
    let font_bold = FontRef::try_from_slice(FONT_BOLD_BYTES).expect("Failed to load bold font");

    draw_header(&mut img, &font_bold, stats, config, battery);
    draw_distance_section(&mut img, &font, &font_bold, stats, config);
    draw_stats_columns(&mut img, &font, &font_bold, stats);
    draw_latest_activity(&mut img, &font, &font_bold, stats);
    draw_polyline(&mut img, stats);

    img
}

fn draw_header(
    img: &mut RgbImage,
    font_bold: &FontRef,
    stats: &DashboardStats,
    _config: &DisplayConfig,
    battery: Option<&BatteryStatus>,
) {
    let header_h = 60;
    draw_filled_rect_mut(img, Rect::at(0, 0).of_size(W, header_h), RED);

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

fn draw_distance_section(
    img: &mut RgbImage,
    font: &FontRef,
    font_bold: &FontRef,
    stats: &DashboardStats,
    config: &DisplayConfig,
) {
    let margin = 24i32;
    let bar_w = (W as i32 - 2 * margin) as u32;
    let bar_h = 22u32;

    // ── Running goal bar ──
    let y = 72;
    draw_goal_bar(
        img,
        font,
        font_bold,
        "🏃  RUN",
        stats.ytd_run_distance_km,
        config.run_goal_km,
        margin,
        y,
        bar_w,
        bar_h,
        RED,
    );

    // ── Cycling goal bar ──
    let y = y + 70;
    draw_goal_bar(
        img,
        font,
        font_bold,
        "🚴  RIDE",
        stats.ytd_ride_distance_km,
        config.ride_goal_km,
        margin,
        y,
        bar_w,
        bar_h,
        BLUE,
    );
}

fn draw_goal_bar(
    img: &mut RgbImage,
    font: &FontRef,
    font_bold: &FontRef,
    label: &str,
    current_km: f64,
    goal_km: f64,
    margin: i32,
    y_start: i32,
    bar_w: u32,
    bar_h: u32,
    accent: Rgb<u8>,
) {
    // Label + value on the same line
    let value_text = format!("{}   {:.0} / {:.0} km", label, current_km, goal_km);
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
    let bar_y = (y_start + 24) as u32;

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

    // Fill
    let pct = if goal_km > 0.0 {
        (current_km / goal_km).min(1.0)
    } else {
        0.0
    };
    let fill_w = ((bar_w - 4) as f64 * pct) as u32;
    if fill_w > 0 {
        draw_filled_rect_mut(
            img,
            Rect::at(margin + 2, bar_y as i32 + 2).of_size(fill_w, bar_h - 4),
            GREEN,
        );
    }

    // Percentage / remaining text
    let pct_text = if current_km >= goal_km {
        let above = current_km - goal_km;
        format!("+{:.0} km above goal", above)
    } else {
        let km_to_go = goal_km - current_km;
        format!("{:.1}%  ·  {:.0} km to go", pct * 100.0, km_to_go)
    };
    draw_text_mut(
        img,
        BLACK,
        margin,
        (bar_y + bar_h + 4) as i32,
        PxScale::from(14.0),
        font,
        &pct_text,
    );
}

fn draw_stats_columns(
    img: &mut RgbImage,
    font: &FontRef,
    font_bold: &FontRef,
    stats: &DashboardStats,
) {
    let y_start = 225;
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
    let count = stats.activity_count();
    draw_text_mut(
        img,
        BLACK,
        margin,
        y_start + 30,
        PxScale::from(22.0),
        font,
        &count.to_string(),
    );

    // Column 2: Total time
    let col2_x = margin + col_w;
    draw_text_mut(
        img,
        RED,
        col2_x,
        y_start,
        PxScale::from(20.0),
        font_bold,
        "TIME",
    );
    draw_text_mut(
        img,
        BLACK,
        col2_x,
        y_start + 30,
        PxScale::from(22.0),
        font,
        &stats.total_time_display(),
    );

    // Column 3: Last Route (polyline)
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
    // The polyline is drawn separately in draw_polyline()
    // We just mark the area here
    let _ = col3_x; // used in draw_polyline
}

fn draw_latest_activity(
    img: &mut RgbImage,
    font: &FontRef,
    font_bold: &FontRef,
    stats: &DashboardStats,
) {
    let y_start = 320;
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

fn draw_polyline(img: &mut RgbImage, stats: &DashboardStats) {
    let points = &stats.last_activity_polyline;
    if points.is_empty() {
        return;
    }

    // Draw area: right third column
    let margin = 24u32;
    let col_w = (W - 2 * margin) / 3;
    let area_x = margin + 2 * col_w;
    let area_y = 250u32;
    let area_w = col_w - 10;
    let area_h = H - area_y - 20;

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
