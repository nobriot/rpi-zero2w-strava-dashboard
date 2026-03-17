use crate::icons;
use crate::ina219::BatteryStatus;
use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
use chrono::{Datelike, Utc};
use image::{Rgb, RgbImage};
use imageproc::drawing::{draw_filled_rect_mut, draw_line_segment_mut, draw_text_mut};
use imageproc::rect::Rect;
use strava::config::GoalConfig;
use strava::stats::DashboardStats;
use strava::types::SportType;

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
const FONT_SYMBOL_BYTES: &[u8] = include_bytes!("../fonts/MesloLGMNerdFont-Bold-subset.ttf");
const FONT_EMOJI_BYTES: &[u8] = include_bytes!("../fonts/NotoEmoji-subset.ttf");
const POWERED_BY_STRAVA: &[u8] = include_bytes!("../assets/powered_by_strava.png");

const MARGIN: i32 = 24;
const HEADER_H: u32 = 56;
const ICON_SZ: u32 = icons::SIZE;

/// Resolution scale factor for rendering.
#[derive(Clone, Copy)]
pub struct Scale(u32);

impl Scale {
  pub fn new(factor: u32) -> Self {
    Scale(factor.max(1))
  }

  pub fn u(&self, v: u32) -> u32 {
    v * self.0
  }

  pub fn i(&self, v: i32) -> i32 {
    v * self.0 as i32
  }

  pub fn f(&self, v: f32) -> f32 {
    v * self.0 as f32
  }

  pub fn px(&self, v: f32) -> PxScale {
    PxScale::from(v * self.0 as f32)
  }

  pub fn factor(&self) -> u32 {
    self.0
  }
}

/// Dashboard display configuration.
pub struct DisplayConfig {
  pub goals: Vec<GoalConfig>,
}

impl Default for DisplayConfig {
  fn default() -> Self {
    Self { goals: vec![GoalConfig { sport: SportType::Ride,
                                    km:    5000.0, },
                       GoalConfig { sport: SportType::Run,
                                    km:    500.0, },
                       GoalConfig { sport: SportType::Swim,
                                    km:    30.0, },], }
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
  bar_section_h:  i32,
  bar_h:          u32,
  lf_entry_h:     i32,
  lf_detail_font: f32,
  lf_name_font:   f32,
}

impl Layout {
  fn compute(stats: &DashboardStats, n_goals: usize, s: Scale) -> Self {
    // With 3 goals, the 2nd and 3rd share a row → 2 visual rows max
    let n_bar_rows = n_goals.min(2) as i32;
    let n_lf = count_longest_fastest_entries(stats) as i32;

    let base_bars = n_bar_rows * 34;
    let base_totals = 34;
    let base_lf = 26 + n_lf * 34;
    let base_last = 60;
    let base_gaps = 24;
    let needed = HEADER_H as i32 + base_bars + base_totals + base_lf + base_last + base_gaps;
    let budget = H as i32;
    let slack = (budget - needed).max(0);

    let bar_extra = (slack / 4).min(14);
    let lf_extra = if n_lf > 0 { (slack / 6).min(8) } else { 0 };

    Layout { bar_section_h:  s.i(34 + bar_extra),
             bar_h:          s.u(14),
             lf_entry_h:     s.i(34 + lf_extra),
             lf_detail_font: s.f(if slack > 60 { 16.0 } else { 15.0 }),
             lf_name_font:   s.f(if slack > 60 { 14.0 } else { 13.0 }), }
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

/// Render the full dashboard as an RGB image, scaled by the given factor.
pub fn render_dashboard(stats: &DashboardStats,
                        battery: Option<&BatteryStatus>,
                        config: &DisplayConfig,
                        avatar: Option<&[u8]>,
                        is_offline: bool,
                        s: Scale)
                        -> RgbImage {
  let mut img = RgbImage::from_pixel(s.u(W), s.u(H), WHITE);

  let font = FontRef::try_from_slice(FONT_BYTES).expect("Failed to load font");
  let font_bold = FontRef::try_from_slice(FONT_BOLD_BYTES).expect("Failed to load bold font");
  let font_symbol = FontRef::try_from_slice(FONT_SYMBOL_BYTES).expect("Failed to load symbol font");
  let font_emoji = FontRef::try_from_slice(FONT_EMOJI_BYTES).expect("Failed to load emoji font");
  let layout = Layout::compute(stats, config.goals.len(), s);

  draw_header(&mut img, &font_bold, stats, avatar, s);

  let y = draw_sport_bars(&mut img, &font, &font_bold, &font_symbol, stats, config, &layout, s);
  let y = draw_totals_row(&mut img, &font, &font_bold, stats, y, s);
  let y = draw_longest_fastest(&mut img,
                               &font,
                               &font_bold,
                               &font_symbol,
                               &font_emoji,
                               stats,
                               y,
                               &layout,
                               s);
  draw_last_activity(&mut img, &font, &font_bold, &font_emoji, stats, y, s);

  draw_battery_indicator(&mut img, &font_bold, battery, is_offline, s);

  img
}

/// Render a minimal offline dashboard indicating no network connectivity.
pub fn render_offline_dashboard(battery: Option<&BatteryStatus>, s: Scale) -> RgbImage {
  let mut img = RgbImage::from_pixel(s.u(W), s.u(H), WHITE);
  let font = FontRef::try_from_slice(FONT_BYTES).expect("Failed to load font");
  let font_bold = FontRef::try_from_slice(FONT_BOLD_BYTES).expect("Failed to load bold font");

  // Orange header bar
  draw_filled_rect_mut(&mut img, Rect::at(0, 0).of_size(s.u(W), s.u(HEADER_H)), ORANGE);

  draw_text_mut(&mut img,
                WHITE,
                s.i(MARGIN),
                s.i(14),
                s.px(26.0),
                &font_bold,
                "STRAVA DASHBOARD");

  // Battery indicator at bottom-right (same as regular dashboard)
  draw_battery_indicator(&mut img, &font_bold, battery, true, s);

  // Centered offline message
  let msg = "OFFLINE";
  let msg_w = approx_text_width(msg, s.u(48));
  let msg_x = (s.u(W) as i32 - msg_w) / 2;
  draw_text_mut(&mut img, ORANGE, msg_x, s.i(180), s.px(48.0), &font_bold, msg);

  let sub = "No internet connection";
  let sub_w = approx_text_width(sub, s.u(20));
  let sub_x = (s.u(W) as i32 - sub_w) / 2;
  draw_text_mut(&mut img, DARK_GRAY, sub_x, s.i(240), s.px(20.0), &font, sub);

  let hint = "Will retry automatically next cycle";
  let hint_w = approx_text_width(hint, s.u(16));
  let hint_x = (s.u(W) as i32 - hint_w) / 2;
  draw_text_mut(&mut img, LIGHT_GRAY, hint_x, s.i(270), s.px(16.0), &font, hint);

  img
}

/// Draw battery percentage and optional "OFFLINE" label at the bottom-right
/// corner.
fn draw_battery_indicator(img: &mut RgbImage,
                          font_bold: &FontRef,
                          battery: Option<&BatteryStatus>,
                          is_offline: bool,
                          s: Scale) {
  let bat_pct = battery.map(|b| b.percentage).unwrap_or(100);
  let bat_fill = bat_pct as f32 / 100.0;
  let bat_text = format!("{}%", bat_pct);
  let text_scale = s.px(14.0);
  let text_w = measure_text_width(font_bold, text_scale, &bat_text) as i32;

  // Stack vertically at bottom-right, left-aligned
  let icon_w = s.i(24);
  let gap = s.i(4);
  let total_w = text_w + gap + icon_w;
  let x = s.u(W) as i32 - total_w;

  if is_offline {
    let label = "OFFLINE";
    let label_scale = s.px(12.0);
    let y_offline = s.u(H) as i32 - s.i(34);
    draw_text_mut(img, DARK_GRAY, x, y_offline, label_scale, font_bold, label);
  }

  let y = s.u(H) as i32 - s.i(18);
  draw_text_mut(img, BLACK, x, y, text_scale, font_bold, &bat_text);
  icons::draw_battery(img, (x + text_w + gap) as u32, y as u32, BLACK, GREEN, bat_fill, s.factor());
}

fn draw_header(img: &mut RgbImage,
               font_bold: &FontRef,
               stats: &DashboardStats,
               avatar: Option<&[u8]>,
               s: Scale) {
  draw_filled_rect_mut(img, Rect::at(0, 0).of_size(s.u(W), s.u(HEADER_H)), ORANGE);

  if let Some(bytes) = avatar {
    draw_avatar(img, bytes, s);
  }

  let year = Utc::now().year();
  let title = format!("{} - {}", stats.athlete_first_name, year);
  let title_scale = s.px(30.0);
  let title_w = measure_text_width(font_bold, title_scale, &title);
  let title_x = ((s.u(W) as f32 - title_w) / 2.0) as i32;
  draw_text_mut(img, WHITE, title_x, s.i(13), title_scale, font_bold, &title);

  draw_powered_by_logo(img, s);
}

fn draw_powered_by_logo(img: &mut RgbImage, s: Scale) {
  let logo = match image::load_from_memory(POWERED_BY_STRAVA) {
    Ok(l) => l,
    Err(_) => return,
  };
  let target_w = s.u(110);
  let aspect = logo.width() as f64 / logo.height() as f64;
  let target_h = (target_w as f64 / aspect) as u32;
  let resized = logo.resize_exact(target_w, target_h, image::imageops::FilterType::Triangle);
  let rgba = resized.to_rgba8();

  let ox = s.u(W) - target_w - s.u(5);
  let oy = s.u(4);

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

fn draw_avatar(img: &mut RgbImage, avatar_bytes: &[u8], s: Scale) {
  let avatar = match image::load_from_memory(avatar_bytes) {
    Ok(a) => a,
    Err(e) => {
      log::warn!("Failed to decode avatar image: {e}");
      return;
    },
  };
  let sz = s.u(AVATAR_SIZE);
  let resized = avatar.resize_exact(sz, sz, image::imageops::FilterType::Triangle);
  let rgb = resized.to_rgb8();
  let cx = sz as f64 / 2.0;
  let cy = sz as f64 / 2.0;
  let r = cx;
  for py in 0..sz {
    for px in 0..sz {
      let dx = px as f64 - cx + 0.5;
      let dy = py as f64 - cy + 0.5;
      if dx * dx + dy * dy <= r * r {
        let pixel = rgb.get_pixel(px, py);
        let ix = s.u(AVATAR_PAD) + px;
        let iy = s.u(AVATAR_PAD) + py;
        if ix < img.width() && iy < img.height() {
          img.put_pixel(ix, iy, *pixel);
        }
      }
    }
  }
}

// --- Sport Goal Bars ---
//
// Layout per full-width bar:
//   [icon] RUN 234km   14h 22m · 27 runs    🏁 2000km
//   [======================== bar ========================]
//
// With 3 goals, the 2nd and 3rd share a row as half-width bars:
//   [icon] RIDE 3456km  85 rides 🏁5000km  [icon] SWIM 28km 12 swims 🏁30km
//   [========= bar =========]               [========= bar =========]

const HALF_BAR_GAP: u32 = 16;

fn draw_sport_bars(img: &mut RgbImage,
                   font: &FontRef,
                   font_bold: &FontRef,
                   font_symbol: &FontRef,
                   stats: &DashboardStats,
                   config: &DisplayConfig,
                   layout: &Layout,
                   s: Scale)
                   -> i32 {
  let full_w = (s.u(W) as i32 - 2 * s.i(MARGIN)) as u32;
  let mut y = (s.u(HEADER_H) + s.u(8)) as i32;

  let goals = &config.goals;
  if goals.is_empty() {
    return y;
  }

  match goals.len() {
    1 | 2 => {
      for goal_cfg in goals {
        draw_goal_bar(img,
                      font,
                      font_bold,
                      font_symbol,
                      stats,
                      goal_cfg,
                      layout,
                      s.i(MARGIN),
                      full_w,
                      s.f(16.0),
                      s.f(14.0),
                      s.f(14.0),
                      s.f(18.0),
                      y,
                      s);
        y += layout.bar_section_h;
      }
    },
    _ => {
      // First goal: full-width
      draw_goal_bar(img,
                    font,
                    font_bold,
                    font_symbol,
                    stats,
                    &goals[0],
                    layout,
                    s.i(MARGIN),
                    full_w,
                    s.f(16.0),
                    s.f(14.0),
                    s.f(14.0),
                    s.f(18.0),
                    y,
                    s);
      y += layout.bar_section_h;

      // 2nd and 3rd: half-width side by side
      let half_w = (full_w - s.u(HALF_BAR_GAP)) / 2;
      let right_x = s.i(MARGIN) + half_w as i32 + s.u(HALF_BAR_GAP) as i32;

      draw_goal_bar(img,
                    font,
                    font_bold,
                    font_symbol,
                    stats,
                    &goals[1],
                    layout,
                    s.i(MARGIN),
                    half_w,
                    s.f(14.0),
                    s.f(12.0),
                    s.f(12.0),
                    s.f(16.0),
                    y,
                    s);
      draw_goal_bar(img,
                    font,
                    font_bold,
                    font_symbol,
                    stats,
                    &goals[2],
                    layout,
                    right_x,
                    half_w,
                    s.f(14.0),
                    s.f(12.0),
                    s.f(12.0),
                    s.f(16.0),
                    y,
                    s);
      y += layout.bar_section_h;
    },
  }

  y
}

/// Draw a single goal bar (full-width or half-width) at the given position.
fn draw_goal_bar(img: &mut RgbImage,
                 font: &FontRef,
                 font_bold: &FontRef,
                 font_symbol: &FontRef,
                 stats: &DashboardStats,
                 goal_cfg: &GoalConfig,
                 layout: &Layout,
                 x: i32,
                 bar_w: u32,
                 left_font_sz: f32,
                 center_font_sz: f32,
                 goal_font_sz: f32,
                 flag_font_sz: f32,
                 y: i32,
                 s: Scale) {
  let sport = goal_cfg.sport;
  let goal = goal_cfg.km;
  let summary = stats.sports.iter().find(|s| s.sport == sport);
  let ytd_km = summary.map(|s| s.ytd_distance_km).unwrap_or(0.0);
  let ytd_count = summary.map(|s| s.ytd_count).unwrap_or(0);
  let ytd_time = summary.map(|s| s.ytd_time_display.as_str()).unwrap_or("0h 00m");
  let noun = sport_count_noun(sport);

  let pct = if goal > 0.0 { (ytd_km / goal).min(1.0) } else { 0.0 };

  // Sport icon
  icons::draw_sport_icon(img, x as u32, (y + s.i(1)) as u32, sport, BLACK, s.factor());

  // Left: "RUN 234km"
  let label = sport_label(sport);
  let left_text = format!("{}  {:.0}km", label, ytd_km);
  let left_scale = PxScale::from(left_font_sz);
  draw_text_mut(img,
                BLACK,
                x + s.u(ICON_SZ) as i32 + s.i(6),
                y + s.i(2),
                left_scale,
                font_bold,
                &left_text);

  // Right: flag + goal (green when reached)
  let goal_reached = ytd_km >= goal;
  let flag_color = if goal_reached { GREEN } else { DARK_GRAY };
  let goal_text = format!("{:.0}km", goal);
  let goal_scale = PxScale::from(goal_font_sz);
  let goal_w = measure_text_width(font, goal_scale, &goal_text) as i32;
  let flag_scale = PxScale::from(flag_font_sz);
  let flag_w = measure_text_width(font_symbol, flag_scale, "\u{F11E} ") as i32;
  let flag_x = x + bar_w as i32 - goal_w - flag_w - s.i(4);
  draw_text_mut(img, flag_color, flag_x, y + s.i(1), flag_scale, font_symbol, "\u{F11E} ");
  draw_text_mut(img,
                flag_color,
                x + bar_w as i32 - goal_w,
                y + s.i(3),
                goal_scale,
                font,
                &goal_text);

  // Center: time + count (condensed for narrow bars)
  let center_scale = PxScale::from(center_font_sz);
  let center_text = if goal_reached {
    format!("+{:.0}km · {} · {} {}", ytd_km - goal, ytd_time, ytd_count, noun)
  } else {
    format!("{} · {} {}", ytd_time, ytd_count, noun)
  };

  let left_end = x
                 + s.u(ICON_SZ) as i32
                 + s.i(6)
                 + measure_text_width(font_bold, left_scale, &left_text) as i32
                 + s.i(8);
  let right_start = flag_x - s.i(6);
  let center_w = measure_text_width(font, center_scale, &center_text) as i32;
  let available = right_start - left_end;

  if available >= center_w {
    let center_x = left_end + (available - center_w) / 2;
    draw_text_mut(img, DARK_GRAY, center_x, y + s.i(3), center_scale, font, &center_text);
  } else {
    // Fall back to count only for very narrow bars
    let short_text = format!("{} {}", ytd_count, noun);
    let short_w = measure_text_width(font, center_scale, &short_text) as i32;
    if available >= short_w {
      let cx = left_end + (available - short_w) / 2;
      draw_text_mut(img, DARK_GRAY, cx, y + s.i(3), center_scale, font, &short_text);
    }
  }

  // Progress bar with black border
  let bar_y = y + s.i(22);
  draw_filled_rect_mut(img, Rect::at(x, bar_y).of_size(bar_w, layout.bar_h), BAR_BG);

  let fill_w = ((bar_w as f64) * pct) as u32;
  if fill_w > 0 {
    draw_filled_rect_mut(img, Rect::at(x, bar_y).of_size(fill_w, layout.bar_h), GREEN);
  }

  // Thin black border
  let bx = x as f32;
  let by = bar_y as f32;
  let bx2 = (x as u32 + bar_w) as f32;
  let by2 = (bar_y as u32 + layout.bar_h) as f32;
  draw_line_segment_mut(img, (bx, by), (bx2, by), BLACK);
  draw_line_segment_mut(img, (bx, by2), (bx2, by2), BLACK);
  draw_line_segment_mut(img, (bx, by), (bx, by2), BLACK);
  draw_line_segment_mut(img, (bx2, by), (bx2, by2), BLACK);

  // Orange dashed year-progress marker
  let yp = year_progress();
  let marker_x = x as f32 + (bar_w as f64 * yp) as f32;
  let bar_top = bar_y as f32;
  let bar_bot = (bar_y as u32 + layout.bar_h) as f32;
  let mut dy = bar_top;
  while dy < bar_bot {
    let seg_end = (dy + s.f(3.0)).min(bar_bot);
    for offset in 0..s.factor() {
      draw_line_segment_mut(img,
                            (marker_x + offset as f32, dy),
                            (marker_x + offset as f32, seg_end),
                            ORANGE);
    }
    dy += s.f(5.0);
  }
}

// --- Totals (single line) ---

fn draw_totals_row(img: &mut RgbImage,
                   font: &FontRef,
                   font_bold: &FontRef,
                   stats: &DashboardStats,
                   y_start: i32,
                   s: Scale)
                   -> i32 {
  const TOTALS: &str = "TOTALS";
  let content_w = (s.u(W) as i32 - 2 * s.i(MARGIN)) as u32;

  // Extra space before separator
  let sep_y = y_start + s.i(4);
  draw_filled_rect_mut(img, Rect::at(s.i(MARGIN), sep_y).of_size(content_w, s.u(1)), LIGHT_GRAY);

  // Chart icon + "TOTALS" in orange, rest in black — centered as a single line
  let y = sep_y + s.i(8);
  icons::draw_bar_chart(img, s.u(MARGIN as u32), (y - s.i(6)) as u32, ORANGE, s.factor());
  let icon_w = s.u(ICON_SZ) as i32 + s.i(4);
  draw_text_mut(img, ORANGE, s.i(MARGIN) + icon_w, y, s.px(18.0), font_bold, TOTALS);

  let center_text = format!("{} activities  ·  {:.0}km  ·  {}  ·  {:.0}m ↑  ·  {} kudos",
                            stats.activity_count,
                            stats.total_distance_km,
                            stats.total_time_display(),
                            stats.total_elevation_gain_m,
                            stats.total_kudos,);
  let text_w = measure_text_width(font, s.px(16.0), &center_text) as i32;
  let center_x = (s.u(W) as i32 - text_w) / 2;
  draw_text_mut(img, BLACK, center_x, y, s.px(16.0), font, &center_text);

  // Extra space after
  y + s.i(28)
}

// --- Longest / Fastest split ---
fn draw_longest_fastest(img: &mut RgbImage,
                        font: &FontRef,
                        font_bold: &FontRef,
                        font_symbol: &FontRef,
                        font_emoji: &FontRef,
                        stats: &DashboardStats,
                        y_start: i32,
                        layout: &Layout,
                        s: Scale)
                        -> i32 {
  let content_w = (s.u(W) as i32 - 2 * s.i(MARGIN)) as u32;
  let half_w = content_w / 2;

  let sep_y = y_start + s.i(2);
  draw_filled_rect_mut(img, Rect::at(s.i(MARGIN), sep_y).of_size(content_w, s.u(1)), LIGHT_GRAY);

  let y = sep_y + s.i(6);
  let detail_sz = PxScale::from(layout.lf_detail_font);
  let name_sz = PxScale::from(layout.lf_name_font);
  let entry_h = layout.lf_entry_h;

  // Left: LONGEST (ruler icon)
  let section_icon_scale = s.px(20.0);
  icons::draw_ruler(img, s.u(MARGIN as u32), y as u32, ORANGE, s.factor());
  draw_text_mut(img,
                ORANGE,
                s.i(MARGIN) + s.u(ICON_SZ) as i32 + s.i(4),
                y,
                s.px(20.0),
                font_bold,
                "LONGEST");

  let mut left_y = y + s.i(26);
  for sp in &stats.sports {
    icons::draw_sport_icon(img,
                           (s.i(MARGIN) + s.i(4)) as u32,
                           left_y as u32,
                           sp.sport,
                           BLACK,
                           s.factor());
    if let Some(ref longest) = sp.longest {
      let line1 = format!("{:.1}km  ·  {}  ·  {}",
                          longest.distance_km, longest.moving_time_display, longest.pace_or_speed);
      draw_text_mut(img,
                    BLACK,
                    s.i(MARGIN) + s.u(ICON_SZ) as i32 + s.i(12),
                    left_y + s.i(2),
                    detail_sz,
                    font_bold,
                    &line1);
      let line2 = format!("{}  ·  {}", truncate_str(&longest.name, 32), longest.date);
      draw_text_with_fallback(img,
                              DARK_GRAY,
                              s.i(MARGIN) + s.u(ICON_SZ) as i32 + s.i(12),
                              left_y + s.i(20),
                              name_sz,
                              font,
                              font_emoji,
                              &line2);
    } else if stats.show_all_sports {
      draw_text_mut(img,
                    LIGHT_GRAY,
                    s.i(MARGIN) + s.u(ICON_SZ) as i32 + s.i(12),
                    left_y + s.i(2),
                    detail_sz,
                    font,
                    "—");
    }
    left_y += entry_h;
  }

  // Right: FASTEST (bolt icon, run race bests — always 3 buckets)
  let right_x = s.i(MARGIN) + half_w as i32 + s.i(12);
  draw_text_mut(img, ORANGE, right_x, y, section_icon_scale, font_symbol, "\u{F0E7} ");
  let bolt_icon_w =
    measure_text_width(font_symbol, section_icon_scale, "\u{F0E7} ") as i32 + s.i(4);
  draw_text_mut(img, ORANGE, right_x + bolt_icon_w, y, s.px(20.0), font_bold, "FASTEST");

  let mut right_y = y + s.i(26);

  for rb in &stats.run_race_bests {
    icons::draw_runner(img, (right_x + s.i(4)) as u32, right_y as u32, BLACK, s.factor());
    if let (Some(pace), Some(dist), Some(time)) =
      (&rb.pace, rb.distance_km, &rb.moving_time_display)
    {
      let line1 = format!("{}  —  {}  ·  {:.1}km  ·  {}", rb.label, pace, dist, time);
      draw_text_mut(img,
                    BLACK,
                    right_x + s.u(ICON_SZ) as i32 + s.i(12),
                    right_y + s.i(2),
                    detail_sz,
                    font_bold,
                    &line1);
      let name = rb.name.as_deref().unwrap_or("—");
      let date = rb.date.as_deref().unwrap_or("—");
      let line2 = format!("{}  ·  {}", truncate_str(name, 30), date);
      draw_text_with_fallback(img,
                              DARK_GRAY,
                              right_x + s.u(ICON_SZ) as i32 + s.i(12),
                              right_y + s.i(20),
                              name_sz,
                              font,
                              font_emoji,
                              &line2);
    } else {
      let line1 = format!("{}  —  —", rb.label);
      draw_text_mut(img,
                    LIGHT_GRAY,
                    right_x + s.u(ICON_SZ) as i32 + s.i(12),
                    right_y + s.i(2),
                    detail_sz,
                    font_bold,
                    &line1);
    }
    right_y += entry_h;
  }

  // Vertical divider
  let div_x = (s.i(MARGIN) + half_w as i32) as f32;
  draw_line_segment_mut(img, (div_x, y as f32), (div_x, left_y.max(right_y) as f32), LIGHT_GRAY);

  left_y.max(right_y) + s.i(4)
}

// --- Last Activity ---

fn draw_last_activity(img: &mut RgbImage,
                      font: &FontRef,
                      font_bold: &FontRef,
                      font_emoji: &FontRef,
                      stats: &DashboardStats,
                      y_start: i32,
                      s: Scale) {
  let content_w = (s.u(W) as i32 - 2 * s.i(MARGIN)) as u32;

  let sep_y = y_start + s.i(2);
  draw_filled_rect_mut(img, Rect::at(s.i(MARGIN), sep_y).of_size(content_w, s.u(1)), LIGHT_GRAY);

  let y = sep_y + s.i(6);

  if let Some(ref last) = stats.last_activity {
    // "LAST ACTIVITY" title
    draw_text_mut(img, ORANGE, s.i(MARGIN), y, s.px(18.0), font_bold, "LAST ACTIVITY");

    // First line: sport icon + name · date
    let line1_x = s.i(MARGIN);
    icons::draw_sport_icon(img,
                           line1_x as u32,
                           (y + s.i(22)) as u32,
                           last.sport,
                           BLACK,
                           s.factor());
    let line1 = format!("{}  ·  {}", truncate_str(&last.name, 30), last.date);
    draw_text_with_fallback(img,
                            BLACK,
                            line1_x + s.u(ICON_SZ) as i32 + s.i(6),
                            y + s.i(24),
                            s.px(16.0),
                            font,
                            font_emoji,
                            &line1);

    let line2 = format!("{:.1}km  ·  {}  ·  {}  ·  {} kudos",
                        last.distance_km, last.pace_or_speed, last.moving_time_display, last.kudos);
    draw_text_mut(img,
                  DARK_GRAY,
                  line1_x + s.u(ICON_SZ) as i32 + s.i(6),
                  y + s.i(44),
                  s.px(15.0),
                  font,
                  &line2);

    // Polyline immediately right of text
    let line1_w = s.u(ICON_SZ) as i32 + s.i(6) + approx_text_width(&line1, s.u(16));
    let line2_w = s.u(ICON_SZ) as i32 + s.i(6) + approx_text_width(&line2, s.u(15));
    let text_right = s.i(MARGIN) + line1_w.max(line2_w) + s.i(20);
    draw_polyline(img, stats, y, text_right, s);
  }
}

// --- Polyline (orange, right of last-activity text) ---

fn draw_polyline(img: &mut RgbImage, stats: &DashboardStats, y_start: i32, x_start: i32, s: Scale) {
  let points = &stats.last_activity_polyline;
  if points.is_empty() {
    return;
  }

  let area_x = x_start.max(s.i(MARGIN)) as u32;
  let area_y = y_start as u32;
  let area_w = (s.u(W) - s.u(8)).saturating_sub(area_x);
  let area_h: u32 = s.u(H).saturating_sub(area_y + s.u(8));

  if area_h < s.u(20) || area_w < s.u(20) {
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
    let y0 = area_y as f32 + off_y as f32 + ((1.0 - (lat0 - min_lat) / lat_range) * draw_h) as f32;
    let x1 = area_x as f32 + off_x as f32 + ((lon1 - min_lon) / lon_range * draw_w) as f32;
    let y1 = area_y as f32 + off_y as f32 + ((1.0 - (lat1 - min_lat) / lat_range) * draw_h) as f32;

    for offset in 0..s.factor() {
      draw_line_segment_mut(img, (x0 + offset as f32, y0), (x1 + offset as f32, y1), ORANGE);
    }
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

/// Draw text with emoji fallback. For each character, tries the primary font
/// first; if the glyph is missing (`.notdef`), falls back to the emoji font.
/// Characters missing from both fonts are silently skipped.
fn draw_text_with_fallback(img: &mut RgbImage,
                           color: Rgb<u8>,
                           x: i32,
                           y: i32,
                           scale: PxScale,
                           font: &FontRef,
                           font_emoji: &FontRef,
                           text: &str) {
  let notdef = ab_glyph::GlyphId(0);
  let mut cursor_x = x as f32;

  for c in text.chars() {
    let gid = font.glyph_id(c);
    if gid != notdef {
      let s: String = c.to_string();
      draw_text_mut(img, color, cursor_x as i32, y, scale, font, &s);
      cursor_x += font.as_scaled(scale).h_advance(gid);
    } else {
      let emoji_gid = font_emoji.glyph_id(c);
      if emoji_gid != notdef {
        let s: String = c.to_string();
        draw_text_mut(img, color, cursor_x as i32, y, scale, font_emoji, &s);
        cursor_x += font_emoji.as_scaled(scale).h_advance(emoji_gid);
      }
      // else: skip the character entirely (no more squares)
    }
  }
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
