use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
use chrono::{Datelike, Utc};
use image::{Rgb, RgbImage};
use imageproc::drawing::{draw_filled_rect_mut, draw_line_segment_mut, draw_text_mut};
use imageproc::rect::Rect;
use strava::stats::DashboardStats;
use strava::types::SportType;

use crate::icons;
use crate::ina219::BatteryStatus;

const W: u32 = 800;
const H: u32 = 480;

const WHITE: Rgb<u8> = Rgb([255, 255, 255]);
const BLACK: Rgb<u8> = Rgb([0, 0, 0]);
const GREEN: Rgb<u8> = Rgb([0, 150, 0]);
const ORANGE: Rgb<u8> = Rgb([252, 76, 2]);
const LIGHT_GRAY: Rgb<u8> = Rgb([210, 210, 210]);
const DARK_GRAY: Rgb<u8> = Rgb([90, 90, 90]);
const BAR_BG: Rgb<u8> = Rgb([230, 230, 230]);

const FONT_BYTES: &[u8] = include_bytes!("../fonts/Inter-Regular.ttf");
const FONT_BOLD_BYTES: &[u8] = include_bytes!("../fonts/Inter-Bold.ttf");
const POWERED_BY_STRAVA: &[u8] = include_bytes!("../assets/powered_by_strava.png");

const MARGIN: i32 = 24;
const HEADER_H: u32 = 56;
const ICON_SZ: u32 = icons::SIZE;

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

fn sport_label(sport: SportType) -> &'static str {
    match sport {
        SportType::Run => "RUN",
        SportType::Ride => "RIDE",
        SportType::Swim => "SWIM",
    }
}

fn sport_count_noun(sport: SportType) -> &'static str {
    match sport {
        SportType::Run => "runs",
        SportType::Ride => "rides",
        SportType::Swim => "swims",
    }
}

fn year_progress() -> f64 {
    let now = Utc::now();
    let day = now.ordinal() as f64;
    let days_in_year = if now.year() % 4 == 0 { 366.0 } else { 365.0 };
    (day / days_in_year).min(1.0)
}

/// Dynamic vertical spacing.
struct Layout {
    bar_section_h: i32,
    bar_h: u32,
    lf_entry_h: i32,
    lf_detail_font: f32,
    lf_name_font: f32,
}

impl Layout {
    fn compute(stats: &DashboardStats) -> Self {
        let n = stats.sports.len() as i32;
        let n_lf = count_longest_fastest_entries(stats) as i32;

        let base_bars = n * 34;
        let base_totals = 34;
        let base_lf = 26 + n_lf * 34;
        let base_last = 60;
        let base_gaps = 24;
        let needed = HEADER_H as i32 + base_bars + base_totals + base_lf + base_last + base_gaps;
        let budget = H as i32;
        let slack = (budget - needed).max(0);

        let bar_extra = (slack / 4).min(14);
        let lf_extra = if n_lf > 0 { (slack / 6).min(8) } else { 0 };

        Layout {
            bar_section_h: 34 + bar_extra,
            bar_h: 14,
            lf_entry_h: 34 + lf_extra,
            lf_detail_font: if slack > 60 { 16.0 } else { 15.0 },
            lf_name_font: if slack > 60 { 14.0 } else { 13.0 },
        }
    }
}

fn count_longest_fastest_entries(stats: &DashboardStats) -> usize {
    let longest_count = if stats.show_all_sports {
        stats.sports.len()
    } else {
        stats.sports.iter().filter(|s| s.longest.is_some()).count()
    };
    // Fastest: always 3 race buckets (5K, 10K, HM)
    let fastest_count = stats.run_race_bests.len();
    longest_count.max(fastest_count)
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
    let layout = Layout::compute(stats);

    draw_header(&mut img, &font_bold, stats, battery, avatar);

    // FIXME: 3 sports bars don't fit, perhaps make a more compact version of the
    // layout where there are three of them.
    let y = draw_sport_bars(&mut img, &font, &font_bold, stats, config, &layout);
    let y = draw_totals_row(&mut img, &font, &font_bold, stats, y);
    let y = draw_longest_fastest(&mut img, &font, &font_bold, stats, y, &layout);
    draw_last_activity(&mut img, &font, &font_bold, stats, y);

    img
}

/// Render a minimal offline dashboard indicating no network connectivity.
pub fn render_offline_dashboard(battery: Option<&BatteryStatus>) -> RgbImage {
    let mut img = RgbImage::from_pixel(W, H, WHITE);
    let font = FontRef::try_from_slice(FONT_BYTES).expect("Failed to load font");
    let font_bold = FontRef::try_from_slice(FONT_BOLD_BYTES).expect("Failed to load bold font");

    // Orange header bar
    draw_filled_rect_mut(&mut img, Rect::at(0, 0).of_size(W, HEADER_H), ORANGE);

    draw_text_mut(
        &mut img,
        WHITE,
        MARGIN,
        14,
        PxScale::from(26.0),
        &font_bold,
        "STRAVA DASHBOARD",
    );

    // Battery icon if available
    if let Some(bat) = battery {
        icons::draw_battery(
            &mut img,
            (W - 70) as u32,
            16,
            WHITE,
            bat.percentage as f32 / 100.0,
        );
    }

    // Centered offline message
    let msg = "OFFLINE";
    let msg_w = approx_text_width(msg, 48);
    let msg_x = (W as i32 - msg_w) / 2;
    draw_text_mut(
        &mut img,
        ORANGE,
        msg_x,
        180,
        PxScale::from(48.0),
        &font_bold,
        msg,
    );

    let sub = "No internet connection";
    let sub_w = approx_text_width(sub, 20);
    let sub_x = (W as i32 - sub_w) / 2;
    draw_text_mut(
        &mut img,
        DARK_GRAY,
        sub_x,
        240,
        PxScale::from(20.0),
        &font,
        sub,
    );

    let hint = "Will retry automatically next cycle";
    let hint_w = approx_text_width(hint, 16);
    let hint_x = (W as i32 - hint_w) / 2;
    draw_text_mut(
        &mut img,
        LIGHT_GRAY,
        hint_x,
        270,
        PxScale::from(16.0),
        &font,
        hint,
    );

    img
}

fn draw_header(
    img: &mut RgbImage,
    font_bold: &FontRef,
    stats: &DashboardStats,
    battery: Option<&BatteryStatus>,
    avatar: Option<&[u8]>,
) {
    draw_filled_rect_mut(img, Rect::at(0, 0).of_size(W, HEADER_H), ORANGE);

    if let Some(bytes) = avatar {
        draw_avatar(img, bytes);
    }

    let year = Utc::now().year();
    let title = format!("{} - {}", stats.athlete_first_name, year);
    draw_text_mut(
        img,
        WHITE,
        center_x_text(W, &title, 30),
        13,
        PxScale::from(30.0),
        font_bold,
        &title,
    );

    draw_powered_by_logo(img);

    if let Some(bat) = battery {
        let bx = W - 60;
        icons::draw_battery(img, bx, 20, WHITE, bat.percentage as f32 / 100.0);
        let bat_text = format!("{}%", bat.percentage);
        draw_text_mut(
            img,
            WHITE,
            (bx - 40) as i32,
            19,
            PxScale::from(18.0),
            font_bold,
            &bat_text,
        );
    }
}

fn draw_powered_by_logo(img: &mut RgbImage) {
    let logo = match image::load_from_memory(POWERED_BY_STRAVA) {
        Ok(l) => l,
        Err(_) => return,
    };
    let target_w = 110u32;
    let aspect = logo.width() as f64 / logo.height() as f64;
    let target_h = (target_w as f64 / aspect) as u32;
    let resized = logo.resize_exact(target_w, target_h, image::imageops::FilterType::Triangle);
    let rgba = resized.to_rgba8();

    let ox = W - target_w - 10;
    let oy = (HEADER_H - target_h) / 2;

    for py in 0..target_h {
        for px in 0..target_w {
            let p = rgba.get_pixel(px, py);
            let alpha = p[3] as f32 / 255.0;
            if alpha < 0.1 {
                continue;
            }
            let ix = ox + px;
            let iy = oy + py;
            if ix < img.width() && iy < img.height() {
                let bg = img.get_pixel(ix, iy);
                let r = (p[0] as f32 * alpha + bg[0] as f32 * (1.0 - alpha)) as u8;
                let g = (p[1] as f32 * alpha + bg[1] as f32 * (1.0 - alpha)) as u8;
                let b = (p[2] as f32 * alpha + bg[2] as f32 * (1.0 - alpha)) as u8;
                img.put_pixel(ix, iy, Rgb([r, g, b]));
            }
        }
    }
}

const AVATAR_SIZE: u32 = 42;
const AVATAR_PAD: u32 = 7;

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
    let cx = AVATAR_SIZE as f64 / 2.0;
    let cy = AVATAR_SIZE as f64 / 2.0;
    let r = cx;
    for py in 0..AVATAR_SIZE {
        for px in 0..AVATAR_SIZE {
            let dx = px as f64 - cx + 0.5;
            let dy = py as f64 - cy + 0.5;
            if dx * dx + dy * dy <= r * r {
                let pixel = rgb.get_pixel(px, py);
                let ix = AVATAR_PAD + px;
                let iy = AVATAR_PAD + py;
                if ix < img.width() && iy < img.height() {
                    img.put_pixel(ix, iy, *pixel);
                }
            }
        }
    }
}

// --- Sport Goal Bars ---
//
// Layout per bar:
//   [icon] RUN 234km    18.1% · 14h 22m · 27 runs    🏁 2000km
//   [======================== bar ========================]

fn draw_sport_bars(
    img: &mut RgbImage,
    font: &FontRef,
    font_bold: &FontRef,
    stats: &DashboardStats,
    config: &DisplayConfig,
    layout: &Layout,
) -> i32 {
    let bar_w = (W as i32 - 2 * MARGIN) as u32;
    let mut y = (HEADER_H + 8) as i32;

    for summary in &stats.sports {
        let goal = config.goal_for(summary.sport);
        let noun = sport_count_noun(summary.sport);
        let pct = if goal > 0.0 {
            (summary.ytd_distance_km / goal).min(1.0)
        } else {
            0.0
        };

        // Sport icon (always black)
        icons::draw_sport_icon(img, MARGIN as u32, (y + 1) as u32, summary.sport, BLACK);

        // Left: "RUN 234km"
        let label = sport_label(summary.sport);
        let left_text = format!("{}  {:.0}km", label, summary.ytd_distance_km);
        draw_text_mut(
            img,
            BLACK,
            MARGIN + ICON_SZ as i32 + 6,
            y + 2,
            PxScale::from(16.0),
            font_bold,
            &left_text,
        );

        // Right: checkered flag + goal
        let goal_text = format!("{:.0}km", goal);
        let goal_w = approx_text_width(&goal_text, 14);
        let flag_x = (MARGIN + bar_w as i32 - goal_w - 16) as u32;
        icons::draw_checkered_flag(img, flag_x, (y + 3) as u32, DARK_GRAY);
        draw_text_mut(
            img,
            DARK_GRAY,
            MARGIN + bar_w as i32 - goal_w,
            y + 3,
            PxScale::from(14.0),
            font,
            &goal_text,
        );

        // Center: "18.1% · 14h 22m · 27 runs"
        let center_text = if summary.ytd_distance_km >= goal {
            format!(
                "+{:.0}km  ·  {}  ·  {} {}",
                summary.ytd_distance_km - goal,
                summary.ytd_time_display,
                summary.ytd_count,
                noun
            )
        } else {
            format!(
                "{:.1}%  ·  {}  ·  {} {}",
                pct * 100.0,
                summary.ytd_time_display,
                summary.ytd_count,
                noun
            )
        };
        let left_end = MARGIN + ICON_SZ as i32 + 6 + approx_text_width(&left_text, 16) + 12;
        let right_start = flag_x as i32 - 8;
        let center_w = approx_text_width(&center_text, 14);
        let center_x = left_end + (right_start - left_end - center_w) / 2;
        draw_text_mut(
            img,
            DARK_GRAY,
            center_x,
            y + 3,
            PxScale::from(14.0),
            font,
            &center_text,
        );

        // Bar with black border
        let bar_y = y + 22;
        draw_filled_rect_mut(
            img,
            Rect::at(MARGIN, bar_y).of_size(bar_w, layout.bar_h),
            BAR_BG,
        );

        let fill_w = ((bar_w as f64) * pct) as u32;
        if fill_w > 0 {
            draw_filled_rect_mut(
                img,
                Rect::at(MARGIN, bar_y).of_size(fill_w, layout.bar_h),
                GREEN,
            );
        }

        // Thin black border
        let bx = MARGIN as f32;
        let by = bar_y as f32;
        let bx2 = (MARGIN as u32 + bar_w) as f32;
        let by2 = (bar_y as u32 + layout.bar_h) as f32;
        draw_line_segment_mut(img, (bx, by), (bx2, by), BLACK);
        draw_line_segment_mut(img, (bx, by2), (bx2, by2), BLACK);
        draw_line_segment_mut(img, (bx, by), (bx, by2), BLACK);
        draw_line_segment_mut(img, (bx2, by), (bx2, by2), BLACK);

        // Orange dashed year-progress marker
        let yp = year_progress();
        let marker_x = MARGIN as f32 + (bar_w as f64 * yp) as f32;
        let bar_top = bar_y as f32;
        let bar_bot = (bar_y as u32 + layout.bar_h) as f32;
        let mut dy = bar_top;
        while dy < bar_bot {
            let seg_end = (dy + 3.0).min(bar_bot);
            draw_line_segment_mut(img, (marker_x, dy), (marker_x, seg_end), ORANGE);
            draw_line_segment_mut(img, (marker_x + 1.0, dy), (marker_x + 1.0, seg_end), ORANGE);
            dy += 5.0;
        }

        y += layout.bar_section_h;
    }

    y
}

// --- Totals (single line) ---

fn draw_totals_row(
    img: &mut RgbImage,
    font: &FontRef,
    font_bold: &FontRef,
    stats: &DashboardStats,
    y_start: i32,
) -> i32 {
    const TOTALS: &str = "TOTALS";
    let content_w = (W as i32 - 2 * MARGIN) as u32;

    // Extra space before separator
    let sep_y = y_start + 4;
    draw_filled_rect_mut(
        img,
        Rect::at(MARGIN, sep_y).of_size(content_w, 1),
        LIGHT_GRAY,
    );

    // "TOTALS" in orange, rest in black — centered as a single line
    let y = sep_y + 8;
    draw_text_mut(
        img,
        ORANGE,
        MARGIN,
        y,
        PxScale::from(18.0),
        font_bold,
        TOTALS,
    );

    let center_text = format!(
        "{} activities  ·  {:.0}km  ·  {}  ·  {:.0}m ↑  ·  {} kudos",
        stats.activity_count,
        stats.total_distance_km,
        stats.total_time_display(),
        stats.total_elevation_gain_m,
        stats.total_kudos,
    );
    let totals_end = MARGIN + measure_text_width(font_bold, PxScale::from(18.0), TOTALS) as i32;
    let right_edge = W as i32 - MARGIN;
    let text_w = measure_text_width(font, PxScale::from(16.0), &center_text) as i32;
    let center_x = totals_end + (right_edge - totals_end - text_w) / 2;
    draw_text_mut(
        img,
        BLACK,
        center_x,
        y,
        PxScale::from(16.0),
        font,
        &center_text,
    );

    // Extra space after
    y + 28
}

// --- Longest / Fastest split ---
fn draw_longest_fastest(
    img: &mut RgbImage,
    font: &FontRef,
    font_bold: &FontRef,
    stats: &DashboardStats,
    y_start: i32,
    layout: &Layout,
) -> i32 {
    let content_w = (W as i32 - 2 * MARGIN) as u32;
    let half_w = content_w / 2;

    let sep_y = y_start + 2;
    draw_filled_rect_mut(
        img,
        Rect::at(MARGIN, sep_y).of_size(content_w, 1),
        LIGHT_GRAY,
    );

    let y = sep_y + 6;
    let detail_sz = PxScale::from(layout.lf_detail_font);
    let name_sz = PxScale::from(layout.lf_name_font);
    let entry_h = layout.lf_entry_h;

    // Left: LONGEST
    icons::draw_ruler(img, MARGIN as u32, y as u32, ORANGE);
    draw_text_mut(
        img,
        ORANGE,
        MARGIN + ICON_SZ as i32 + 4,
        y,
        PxScale::from(20.0),
        font_bold,
        "LONGEST",
    );

    let mut left_y = y + 26;
    for s in &stats.sports {
        icons::draw_sport_icon(img, (MARGIN + 4) as u32, left_y as u32, s.sport, BLACK);
        if let Some(ref longest) = s.longest {
            let line1 = format!(
                "{:.1}km  ·  {}  ·  {}",
                longest.distance_km, longest.moving_time_display, longest.pace_or_speed
            );
            draw_text_mut(
                img,
                BLACK,
                MARGIN + ICON_SZ as i32 + 12,
                left_y + 2,
                detail_sz,
                font_bold,
                &line1,
            );
            let line2 = format!("{}  ·  {}", truncate_str(&longest.name, 32), longest.date);
            draw_text_mut(
                img,
                DARK_GRAY,
                MARGIN + ICON_SZ as i32 + 12,
                left_y + 20,
                name_sz,
                font,
                &line2,
            );
        } else if stats.show_all_sports {
            draw_text_mut(
                img,
                LIGHT_GRAY,
                MARGIN + ICON_SZ as i32 + 12,
                left_y + 2,
                detail_sz,
                font,
                "—",
            );
        }
        left_y += entry_h;
    }

    // Right: FASTEST (run race bests — always 3 buckets)
    let right_x = MARGIN + half_w as i32 + 12;
    icons::draw_lightning(img, right_x as u32, y as u32, ORANGE);
    draw_text_mut(
        img,
        ORANGE,
        right_x + ICON_SZ as i32 + 4,
        y,
        PxScale::from(20.0),
        font_bold,
        "FASTEST",
    );

    let mut right_y = y + 26;

    for rb in &stats.run_race_bests {
        icons::draw_runner(img, (right_x + 4) as u32, right_y as u32, BLACK);
        if let (Some(pace), Some(dist), Some(time)) =
            (&rb.pace, rb.distance_km, &rb.moving_time_display)
        {
            let line1 = format!("{}  —  {}  ·  {:.1}km  ·  {}", rb.label, pace, dist, time);
            draw_text_mut(
                img,
                BLACK,
                right_x + ICON_SZ as i32 + 12,
                right_y + 2,
                detail_sz,
                font_bold,
                &line1,
            );
            let name = rb.name.as_deref().unwrap_or("—");
            let date = rb.date.as_deref().unwrap_or("—");
            let line2 = format!("{}  ·  {}", truncate_str(name, 30), date);
            draw_text_mut(
                img,
                DARK_GRAY,
                right_x + ICON_SZ as i32 + 12,
                right_y + 20,
                name_sz,
                font,
                &line2,
            );
        } else {
            let line1 = format!("{}  —  —", rb.label);
            draw_text_mut(
                img,
                LIGHT_GRAY,
                right_x + ICON_SZ as i32 + 12,
                right_y + 2,
                detail_sz,
                font_bold,
                &line1,
            );
        }
        right_y += entry_h;
    }

    // Vertical divider
    let div_x = (MARGIN + half_w as i32) as f32;
    draw_line_segment_mut(
        img,
        (div_x, y as f32),
        (div_x, left_y.max(right_y) as f32),
        LIGHT_GRAY,
    );

    left_y.max(right_y) + 4
}

// --- Last Activity ---

fn draw_last_activity(
    img: &mut RgbImage,
    font: &FontRef,
    font_bold: &FontRef,
    stats: &DashboardStats,
    y_start: i32,
) {
    let content_w = (W as i32 - 2 * MARGIN) as u32;

    let sep_y = y_start + 2;
    draw_filled_rect_mut(
        img,
        Rect::at(MARGIN, sep_y).of_size(content_w, 1),
        LIGHT_GRAY,
    );

    let y = sep_y + 6;

    if let Some(ref last) = stats.last_activity {
        // "LAST ACTIVITY" title
        draw_text_mut(
            img,
            ORANGE,
            MARGIN,
            y,
            PxScale::from(18.0),
            font_bold,
            "LAST ACTIVITY",
        );

        // First line: sport icon + name · date
        let line1_x = MARGIN;
        icons::draw_sport_icon(img, line1_x as u32, (y + 22) as u32, last.sport, BLACK);
        let line1 = format!("{}  ·  {}", truncate_str(&last.name, 30), last.date);
        draw_text_mut(
            img,
            BLACK,
            line1_x + ICON_SZ as i32 + 6,
            y + 24,
            PxScale::from(16.0),
            font,
            &line1,
        );

        let line2 = format!(
            "{:.1}km  ·  {}  ·  {}",
            last.distance_km, last.pace_or_speed, last.moving_time_display,
        );
        draw_text_mut(
            img,
            DARK_GRAY,
            line1_x + ICON_SZ as i32 + 6,
            y + 44,
            PxScale::from(15.0),
            font,
            &line2,
        );

        // Polyline immediately right of text
        let line1_w = ICON_SZ as i32 + 6 + approx_text_width(&line1, 16);
        let line2_w = ICON_SZ as i32 + 6 + approx_text_width(&line2, 15);
        let text_right = MARGIN + line1_w.max(line2_w) + 20;
        draw_polyline(img, stats, y, text_right);
    }
}

// --- Polyline (orange, right of last-activity text) ---

fn draw_polyline(img: &mut RgbImage, stats: &DashboardStats, y_start: i32, x_start: i32) {
    let points = &stats.last_activity_polyline;
    if points.is_empty() {
        return;
    }

    let area_x = x_start.max(MARGIN) as u32;
    let area_y = y_start as u32;
    let area_w = (W - 8).saturating_sub(area_x);
    let area_h: u32 = H.saturating_sub(area_y + 8);

    if area_h < 20 || area_w < 20 {
        return;
    }

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

    for window in points.windows(2) {
        let (lat0, lon0) = window[0];
        let (lat1, lon1) = window[1];

        let x0 = area_x as f32 + off_x as f32 + ((lon0 - min_lon) / lon_range * draw_w) as f32;
        let y0 =
            area_y as f32 + off_y as f32 + ((1.0 - (lat0 - min_lat) / lat_range) * draw_h) as f32;
        let x1 = area_x as f32 + off_x as f32 + ((lon1 - min_lon) / lon_range * draw_w) as f32;
        let y1 =
            area_y as f32 + off_y as f32 + ((1.0 - (lat1 - min_lat) / lat_range) * draw_h) as f32;

        draw_line_segment_mut(img, (x0, y0), (x1, y1), ORANGE);
        draw_line_segment_mut(img, (x0 + 1.0, y0), (x1 + 1.0, y1), ORANGE);
    }
}

// --- Helpers ---

fn measure_text_width(font: &FontRef, scale: PxScale, text: &str) -> f32 {
    let scaled = font.as_scaled(scale);
    let mut width = 0.0;
    let mut prev: Option<ab_glyph::GlyphId> = None;
    for c in text.chars() {
        let gid = font.glyph_id(c);
        if let Some(p) = prev {
            width += scaled.kern(p, gid);
        }
        width += scaled.h_advance(gid);
        prev = Some(gid);
    }
    width
}

fn center_x_text(img_width: u32, text: &str, font_size: u32) -> i32 {
    let approx_char_w = font_size as f32 * 0.55;
    let text_w = text.len() as f32 * approx_char_w;
    ((img_width as f32 - text_w) / 2.0) as i32
}

fn approx_text_width(text: &str, font_size: u32) -> i32 {
    (text.len() as f32 * font_size as f32 * 0.55) as i32
}

fn truncate_str(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars - 1).collect();
        format!("{truncated}…")
    }
}
