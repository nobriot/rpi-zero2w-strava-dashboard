use crate::config::{DisplayConfig, GoalConfig, Orientation};
use crate::icons;
use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
use chrono::{Datelike, Utc};
use common::{BatteryStatus, DashboardStats, SportType};
use image::{Rgb, RgbImage};
use imageproc::drawing::{draw_filled_rect_mut, draw_line_segment_mut, draw_text_mut};
use imageproc::rect::Rect;

const LANDSCAPE_W: u32 = 800;
const LANDSCAPE_H: u32 = 480;

/// Canvas dimensions, derived from the orientation config.
#[derive(Clone, Copy)]
struct Canvas {
  w: u32,
  h: u32,
}

impl Canvas {
  fn from_orientation(o: Orientation) -> Self {
    match o {
      Orientation::Landscape => Canvas { w: LANDSCAPE_W, h: LANDSCAPE_H },
      Orientation::Portrait => Canvas { w: LANDSCAPE_H, h: LANDSCAPE_W },
    }
  }
}

const WHITE: Rgb<u8> = Rgb([255, 255, 255]);
const BLACK: Rgb<u8> = Rgb([0, 0, 0]);
const GREEN: Rgb<u8> = Rgb([0, 150, 0]);
const ORANGE: Rgb<u8> = Rgb([252, 76, 2]);
const RED: Rgb<u8> = Rgb([200, 0, 0]);
// const DARK_GRAY: Rgb<u8> = Rgb([120, 120, 120]);
// const LIGHT_GRAY: Rgb<u8> = Rgb([210, 210, 210]);
const BAR_BG: Rgb<u8> = Rgb([230, 230, 230]);

const FONT_REGULAR_BYTES: &[u8] = include_bytes!("../fonts/Inter-Regular.ttf");
const FONT_SEMIBOLD_BYTES: &[u8] = include_bytes!("../fonts/Inter-SemiBold.otf");
const FONT_BOLD_BYTES: &[u8] = include_bytes!("../fonts/Inter-Bold.ttf");
const FONT_BLACK_BYTES: &[u8] = include_bytes!("../fonts/Inter-Black.otf");
const FONT_SYMBOL_BYTES: &[u8] = include_bytes!("../fonts/MesloLGMNerdFont-Bold-subset.ttf");
const FONT_EMOJI_BYTES: &[u8] = include_bytes!("../fonts/NotoEmoji-subset.ttf");
const POWERED_BY_STRAVA: &[u8] = include_bytes!("../assets/powered_by_strava.png");

const MARGIN: i32 = 24;
const HEADER_H: u32 = 56;
const ICON_SZ: u32 = icons::SIZE;
const DIVIDER_THICKNESS: u32 = 1;
const BAR_BORDER_THICKNESS: u32 = 1;

// Text style hierarchy
const TITLE_COLOR: Rgb<u8> = RED;
const TITLE_FONT_SZ: f32 = 24.0;
const MAIN_COLOR: Rgb<u8> = BLACK;
const MAIN_FONT_SZ: f32 = 18.0;
const SECONDARY_COLOR: Rgb<u8> = BLACK;
const SECONDARY_FONT_SZ: f32 = 18.0;

/// Resolution scale factor for rendering.
#[derive(Clone, Copy)]
pub struct Scale {
  factor: u32,
}

impl Scale {
  pub fn new(factor: u32) -> Self {
    Scale { factor: factor.max(1) }
  }

  pub fn u(&self, v: u32) -> u32 {
    v * self.factor
  }

  pub fn i(&self, v: i32) -> i32 {
    v * self.factor as i32
  }

  pub fn f(&self, v: f32) -> f32 {
    v * self.factor as f32
  }

  pub fn px(&self, v: f32) -> PxScale {
    PxScale::from(v * self.factor as f32)
  }

  pub fn factor(&self) -> u32 {
    self.factor
  }
}

fn sport_label(sport: SportType) -> &'static str {
  match sport {
    SportType::Run => "RUN",
    SportType::Ride => "RIDE",
    SportType::Swim => "SWIM",
    SportType::WeightTraining => "WEIGHTS",
    SportType::Yoga => "YOGA",
    SportType::Pilates => "PILATES",
    SportType::Workout => "WORKOUT",
  }
}

fn sport_count_noun(sport: SportType) -> &'static str {
  match sport {
    SportType::Run => "runs",
    SportType::Ride => "rides",
    SportType::Swim => "swims",
    SportType::WeightTraining | SportType::Yoga | SportType::Pilates | SportType::Workout => {
      "sessions"
    },
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
  fn compute(stats: &DashboardStats,
             n_goals: usize,
             show_totals: bool,
             show_longest_fastest: bool,
             orientation: Orientation,
             c: Canvas,
             s: Scale)
             -> Self {
    let n_lf = count_longest_fastest_entries(stats) as i32;

    // Portrait: all bars stacked; landscape: 2nd+3rd share a row
    let n_bar_rows = match orientation {
      Orientation::Landscape => n_goals.min(2) as i32,
      Orientation::Portrait => n_goals as i32,
    };

    // Portrait: longest/fastest are stacked (both columns), so double the entries
    let lf_rows = match orientation {
      Orientation::Landscape => n_lf,
      Orientation::Portrait => {
        let longest = if stats.show_all_sports {
          stats.sports.len()
        } else {
          stats.sports.iter().filter(|sp| sp.longest.is_some()).count()
        };
        let fastest = stats.run_race_bests.len();
        (longest + fastest) as i32
      },
    };

    let base_bars = n_bar_rows * 34;
    let base_totals = if show_totals { 38 } else { 0 };
    let base_lf = if show_longest_fastest { 30 + lf_rows * 36 } else { 0 };
    let base_last = 64;
    let base_gaps = 32;
    let needed = HEADER_H as i32 + base_bars + base_totals + base_lf + base_last + base_gaps;
    let budget = c.h as i32;
    let slack = (budget - needed).max(0);

    let bar_extra = (slack / 4).min(14);
    let lf_extra = if lf_rows > 0 { (slack / 6).min(8) } else { 0 };

    Layout { bar_section_h:  s.i(34 + bar_extra),
             bar_h:          s.u(16),
             lf_entry_h:     s.i(34 + lf_extra),
             lf_detail_font: s.f(if slack > 60 { 19.0 } else { 18.0 }),
             lf_name_font:   s.f(if slack > 60 { 17.0 } else { 16.0 }), }
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
  let c = Canvas::from_orientation(config.orientation);
  let mut img = RgbImage::from_pixel(s.u(c.w), s.u(c.h), WHITE);

  let (font_bytes, bold_bytes, title_bytes) = if config.light_fonts {
    (FONT_REGULAR_BYTES, FONT_SEMIBOLD_BYTES, FONT_BOLD_BYTES)
  } else {
    (FONT_SEMIBOLD_BYTES, FONT_BOLD_BYTES, FONT_BLACK_BYTES)
  };
  let font = FontRef::try_from_slice(font_bytes).expect("Failed to load font");
  let font_bold = FontRef::try_from_slice(bold_bytes).expect("Failed to load bold font");
  let font_title = FontRef::try_from_slice(title_bytes).expect("Failed to load title font");
  let font_symbol = FontRef::try_from_slice(FONT_SYMBOL_BYTES).expect("Failed to load symbol font");
  let font_emoji = FontRef::try_from_slice(FONT_EMOJI_BYTES).expect("Failed to load emoji font");
  let layout = Layout::compute(stats,
                               config.goals.len(),
                               config.show_totals,
                               config.show_longest_fastest,
                               config.orientation,
                               c,
                               s);

  draw_header(&mut img, &font_title, stats, avatar, config.orientation, c, s);

  let y = draw_sport_bars(&mut img, &font, &font_bold, &font_symbol, stats, config, &layout, c, s);
  let y = if config.show_totals {
    draw_totals_row(&mut img, &font, &font_bold, stats, y, c, s)
  } else {
    y
  };
  let y = if config.show_longest_fastest {
    draw_longest_fastest(&mut img,
                         &font,
                         &font_bold,
                         &font_emoji,
                         stats,
                         y,
                         &layout,
                         c,
                         config.orientation,
                         s)
  } else {
    y
  };
  draw_last_activity(&mut img, &font, &font_bold, &font_emoji, stats, y, is_offline, config, c, s);

  draw_battery_indicator(&mut img, &font, &font_bold, battery, is_offline, config.debug, c, s);

  // Portrait is rendered at 480x800 but the physical display is 800x480.
  // Rotate 90 degrees CW so the image maps correctly to the hardware.
  if config.orientation == Orientation::Portrait {
    image::imageops::rotate90(&img)
  } else {
    img
  }
}

/// Draw battery percentage, optional "OFFLINE" label, and optional debug
/// sync timestamp at the bottom-right corner.
fn draw_battery_indicator(img: &mut RgbImage,
                          font: &FontRef,
                          font_bold: &FontRef,
                          battery: Option<&BatteryStatus>,
                          is_offline: bool,
                          debug: bool,
                          c: Canvas,
                          s: Scale) {
  let bat_pct = battery.map(|b| b.percentage()).unwrap_or(100);
  let bat_fill = bat_pct as f32 / 100.0;
  let bat_text = format!("{}%", bat_pct);
  let text_scale = s.px(16.0);
  let text_w = measure_text_width(font_bold, text_scale, &bat_text) as i32;

  // Stack vertically at bottom-right, left-aligned
  let icon_w = s.i(24);
  let gap = s.i(4);
  let total_w = text_w + gap + icon_w;
  let x = s.u(c.w) as i32 - total_w;

  if debug {
    let now = chrono::Local::now();
    let sync_text = now.format("%d/%m %H:%M").to_string();
    let sync_scale = s.px(12.0);
    let sync_w = measure_text_width(font, sync_scale, &sync_text) as i32;
    let sync_x = s.u(c.w) as i32 - sync_w - 4;
    let sync_y = s.u(c.h) as i32 - s.i(40);
    draw_text_mut(img, BLACK, sync_x, sync_y, sync_scale, font, &sync_text);
  }

  if is_offline {
    let label = "OFFLINE";
    let label_scale = s.px(14.0);
    let y_offline = s.u(c.h) as i32 - s.i(56);
    draw_text_mut(img, BLACK, x, y_offline, label_scale, font_bold, label);
  }

  let y = s.u(c.h) as i32 - s.i(MARGIN);
  draw_text_mut(img, BLACK, x, y, text_scale, font_bold, &bat_text);
  icons::draw_battery(img,
                      (x + text_w + gap) as u32,
                      (y + 2) as u32,
                      BLACK,
                      GREEN,
                      bat_fill,
                      s.factor());
}

fn draw_header(img: &mut RgbImage,
               font_title: &FontRef,
               stats: &DashboardStats,
               avatar: Option<&[u8]>,
               orientation: Orientation,
               c: Canvas,
               s: Scale) {
  draw_filled_rect_mut(img, Rect::at(0, 0).of_size(s.u(c.w), s.u(HEADER_H)), ORANGE);

  let year = Utc::now().year();
  let title = format!("{} - {}", stats.athlete_first_name, year);
  let title_scale = s.px(45.0);
  let title_w = measure_text_width(font_title, title_scale, &title);

  // Measure how much space we need for all header elements
  let avatar_space = if avatar.is_some() { s.u(AVATAR_PAD + AVATAR_SIZE + 4) as f32 } else { 0.0 };
  let logo_space = s.f(120.0);
  let available = s.u(c.w) as f32;

  // Draw what fits: avatar first, logo last
  let show_avatar = avatar.is_some() && avatar_space + title_w + 20.0 <= available;
  let show_logo = orientation != Orientation::Portrait
                  && show_avatar as u32 as f32 * avatar_space + title_w + logo_space + 20.0
                     <= available;

  if show_avatar && let Some(bytes) = avatar {
    draw_avatar(img, bytes, s);
  }

  if orientation == Orientation::Portrait {
    let name = stats.athlete_first_name.as_str();
    let year_text = format!("{}", year);
    let name_w = measure_text_width(font_title, title_scale, name);
    let year_w = measure_text_width(font_title, title_scale, &year_text);
    let name_x = ((available - name_w) / 2.0) as i32;
    let year_x = (available - year_w) as i32 - s.i(5);
    draw_text_mut(img, WHITE, name_x, s.i(6), title_scale, font_title, name);
    draw_text_mut(img, WHITE, year_x, s.i(6), title_scale, font_title, &year_text);
  } else {
    let title_x = ((available - title_w) / 2.0) as i32;
    draw_text_mut(img, WHITE, title_x, s.i(6), title_scale, font_title, &title);
    if show_logo {
      draw_powered_by_logo(img, c, s);
    }
  }
}

fn draw_powered_by_logo(img: &mut RgbImage, c: Canvas, s: Scale) {
  let logo = match image::load_from_memory(POWERED_BY_STRAVA) {
    Ok(l) => l,
    Err(_) => return,
  };
  let target_w = s.u(110);
  let aspect = logo.width() as f64 / logo.height() as f64;
  let target_h = (target_w as f64 / aspect) as u32;
  let resized = logo.resize_exact(target_w, target_h, image::imageops::FilterType::Triangle);
  let rgba = resized.to_rgba8();

  let ox = s.u(c.w) - target_w - s.u(5);
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
                   c: Canvas,
                   s: Scale)
                   -> i32 {
  let full_w = (s.u(c.w) as i32 - 2 * s.i(MARGIN)) as u32;
  let mut y = (s.u(HEADER_H) + s.u(8)) as i32;

  let goals = &config.goals;
  if goals.is_empty() {
    return y;
  }

  let is_portrait = config.orientation == Orientation::Portrait;

  // Portrait: all bars stacked as compact (no sport label)
  if is_portrait || goals.len() <= 2 {
    let show_label = !is_portrait;
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
                    s.f(18.0),
                    s.f(16.0),
                    s.f(16.0),
                    s.f(22.0),
                    show_label,
                    y,
                    s);
      y += layout.bar_section_h;
    }
  } else {
    // Landscape with 3+ goals: first full-width, rest half-width
    draw_goal_bar(img,
                  font,
                  font_bold,
                  font_symbol,
                  stats,
                  &goals[0],
                  layout,
                  s.i(MARGIN),
                  full_w,
                  s.f(18.0),
                  s.f(16.0),
                  s.f(16.0),
                  s.f(22.0),
                  true,
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
                  s.f(18.0),
                  s.f(16.0),
                  s.f(16.0),
                  s.f(22.0),
                  false,
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
                  s.f(18.0),
                  s.f(16.0),
                  s.f(16.0),
                  s.f(22.0),
                  false,
                  y,
                  s);
    y += layout.bar_section_h;
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
                 show_label: bool,
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
  icons::draw_sport_icon(img, x as u32, (y + s.i(1)) as u32, sport, false, BLACK, s.factor());

  // Left: "RUN 234km" (full-width) or "234km" (half-width)
  let left_text = if show_label {
    format!("{}  {:.0} km", sport_label(sport), ytd_km)
  } else {
    format!("{:.0} km", ytd_km)
  };
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
  let flag_color = if goal_reached { GREEN } else { BLACK };
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
    draw_text_mut(img, BLACK, center_x, y + s.i(3), center_scale, font, &center_text);
  } else {
    // Fall back to count only for very narrow bars
    let short_text = format!("{} {}", ytd_count, noun);
    let short_w = measure_text_width(font, center_scale, &short_text) as i32;
    if available >= short_w {
      let cx = left_end + (available - short_w) / 2;
      draw_text_mut(img, BLACK, cx, y + s.i(3), center_scale, font, &short_text);
    }
  }

  // Progress bar with black border
  let bar_y = y + s.i(24);
  draw_filled_rect_mut(img, Rect::at(x, bar_y).of_size(bar_w, layout.bar_h), BAR_BG);

  let fill_w = ((bar_w as f64) * pct) as u32;
  if fill_w > 0 {
    draw_filled_rect_mut(img, Rect::at(x, bar_y).of_size(fill_w, layout.bar_h), GREEN);
  }

  // Black border
  let bt = s.u(BAR_BORDER_THICKNESS);
  draw_filled_rect_mut(img, Rect::at(x, bar_y).of_size(bar_w, bt), BLACK);
  draw_filled_rect_mut(img,
                       Rect::at(x, bar_y + layout.bar_h as i32 - bt as i32).of_size(bar_w, bt),
                       BLACK);
  draw_filled_rect_mut(img, Rect::at(x, bar_y).of_size(bt, layout.bar_h), BLACK);
  draw_filled_rect_mut(img,
                       Rect::at(x + bar_w as i32 - bt as i32, bar_y).of_size(bt, layout.bar_h),
                       BLACK);

  // Red solid year-progress marker
  let yp = year_progress();
  let marker_x = x + (bar_w as f64 * yp) as i32;
  let marker_w = s.u(3);
  draw_filled_rect_mut(img, Rect::at(marker_x, bar_y).of_size(marker_w, layout.bar_h), RED);
}

// --- Totals (single line) ---

fn draw_totals_row(img: &mut RgbImage,
                   font: &FontRef,
                   font_bold: &FontRef,
                   stats: &DashboardStats,
                   y_start: i32,
                   c: Canvas,
                   s: Scale)
                   -> i32 {
  const TOTALS: &str = "TOTALS";
  let content_w = (s.u(c.w) as i32 - 2 * s.i(MARGIN)) as u32;

  // Extra space before separator
  let sep_y = y_start + s.i(8);
  draw_filled_rect_mut(img,
                       Rect::at(s.i(MARGIN), sep_y).of_size(content_w, s.u(DIVIDER_THICKNESS)),
                       BLACK);

  // Chart icon + "TOTALS" in orange, stats in black — left-aligned
  let y = sep_y + s.i(10);
  icons::draw_bar_chart(img, s.u(MARGIN as u32), y as u32, TITLE_COLOR, s.factor());
  let icon_w = s.u(ICON_SZ) as i32 + s.i(4);
  draw_text_mut(img,
                TITLE_COLOR,
                s.i(MARGIN) + icon_w,
                y,
                s.px(TITLE_FONT_SZ),
                font_bold,
                TOTALS);

  let center_text = format!("{} activities  ·  {:.0}km  ·  {}  ·  {:.0}m ↑  ·  {} kudos",
                            stats.activity_count,
                            stats.total_distance_km,
                            stats.total_time_display(),
                            stats.total_elevation_gain_m,
                            stats.total_kudos,);
  let title_w = measure_text_width(font_bold, s.px(TITLE_FONT_SZ), TOTALS) as i32;
  let stats_x = s.i(MARGIN) + icon_w + title_w + s.i(20);
  let baseline_offset = s.i(4);
  draw_text_mut(img,
                SECONDARY_COLOR,
                stats_x,
                y + baseline_offset,
                s.px(SECONDARY_FONT_SZ),
                font,
                &center_text);

  // Extra space after
  y + s.i(32)
}

// --- Longest / Fastest split ---

/// Draw the LONGEST entries starting at (x, y). Returns y after the last entry.
fn draw_longest_entries(img: &mut RgbImage,
                        font: &FontRef,
                        font_bold: &FontRef,
                        font_emoji: &FontRef,
                        stats: &DashboardStats,
                        layout: &Layout,
                        x: i32,
                        mut y: i32,
                        s: Scale)
                        -> i32 {
  let detail_sz = PxScale::from(layout.lf_detail_font);
  let name_sz = PxScale::from(layout.lf_name_font);
  let text_x = x + s.u(ICON_SZ) as i32 + s.i(12);

  for sp in &stats.sports {
    let is_mtb = sp.longest.as_ref().is_some_and(|l| l.is_mtb);
    icons::draw_sport_icon(img, (x + s.i(4)) as u32, y as u32, sp.sport, is_mtb, BLACK, s.factor());
    if let Some(ref longest) = sp.longest {
      let line1 = format!("{:.1}km  ·  {}  ·  {}",
                          longest.distance_km, longest.moving_time_display, longest.pace_or_speed);
      draw_text_mut(img, BLACK, text_x, y + s.i(2), detail_sz, font_bold, &line1);
      let line2 = format!("{}  ·  {}", truncate_str(&longest.name, 32), longest.date);
      draw_text_with_fallback(img, BLACK, text_x, y + s.i(22), name_sz, font, font_emoji, &line2);
    } else if stats.show_all_sports {
      draw_text_mut(img, BLACK, text_x, y + s.i(2), detail_sz, font, "—");
    }
    y += layout.lf_entry_h;
  }
  y
}

/// Draw the FASTEST entries starting at (x, y). Returns y after the last entry.
fn draw_fastest_entries(img: &mut RgbImage,
                        font: &FontRef,
                        font_bold: &FontRef,
                        font_emoji: &FontRef,
                        stats: &DashboardStats,
                        layout: &Layout,
                        x: i32,
                        mut y: i32,
                        s: Scale)
                        -> i32 {
  let detail_sz = PxScale::from(layout.lf_detail_font);
  let name_sz = PxScale::from(layout.lf_name_font);
  let text_x = x + s.u(ICON_SZ) as i32 + s.i(12);

  // Render labels in a fixed-width column so values align across rows.
  let label_col_w = stats.run_race_bests
                         .iter()
                         .map(|rb| measure_text_width(font_bold, detail_sz, rb.label) as i32)
                         .max()
                         .unwrap_or(0);
  let value_x = text_x + label_col_w + s.i(8);

  for rb in &stats.run_race_bests {
    icons::draw_runner(img, (x + s.i(4)) as u32, y as u32, BLACK, s.factor());
    draw_text_mut(img, BLACK, text_x, y + s.i(2), detail_sz, font_bold, rb.label);
    if let (Some(pace), Some(dist), Some(time)) =
      (&rb.pace, rb.distance_km, &rb.moving_time_display)
    {
      let value = format!("-  {}  ·  {}", pace, time);
      draw_text_mut(img, BLACK, value_x, y + s.i(2), detail_sz, font_bold, &value);
      if dist > rb.target_km * 1.1 {
        let value_w = measure_text_width(font_bold, detail_sz, &value) as i32;
        let suffix = format!("  ·  ({:.1}km)", dist);
        draw_text_mut(img, BLACK, value_x + value_w, y + s.i(2), detail_sz, font, &suffix);
      }
      let name = rb.name.as_deref().unwrap_or("—");
      let date = rb.date.as_deref().unwrap_or("—");
      let line2 = format!("{}  ·  {}", truncate_str(name, 30), date);
      draw_text_with_fallback(img, BLACK, text_x, y + s.i(22), name_sz, font, font_emoji, &line2);
    } else {
      draw_text_mut(img, BLACK, value_x, y + s.i(2), detail_sz, font_bold, "—");
    }
    y += layout.lf_entry_h;
  }
  y
}

fn draw_longest_fastest(img: &mut RgbImage,
                        font: &FontRef,
                        font_bold: &FontRef,
                        font_emoji: &FontRef,
                        stats: &DashboardStats,
                        y_start: i32,
                        layout: &Layout,
                        c: Canvas,
                        orientation: Orientation,
                        s: Scale)
                        -> i32 {
  let content_w = (s.u(c.w) as i32 - 2 * s.i(MARGIN)) as u32;

  let sep_y = y_start + s.i(6);
  draw_filled_rect_mut(img,
                       Rect::at(s.i(MARGIN), sep_y).of_size(content_w, s.u(DIVIDER_THICKNESS)),
                       BLACK);

  let y = sep_y + s.i(8);
  let section_title_scale = s.px(TITLE_FONT_SZ);

  match orientation {
    Orientation::Landscape => {
      let half_w = content_w / 2;

      // Left: LONGEST
      icons::draw_ruler(img, s.u(MARGIN as u32), y as u32, TITLE_COLOR, s.factor());
      draw_text_mut(img,
                    TITLE_COLOR,
                    s.i(MARGIN) + s.u(ICON_SZ) as i32 + s.i(4),
                    y,
                    section_title_scale,
                    font_bold,
                    "LONGEST");
      let left_y = draw_longest_entries(img,
                                        font,
                                        font_bold,
                                        font_emoji,
                                        stats,
                                        layout,
                                        s.i(MARGIN),
                                        y + s.i(26),
                                        s);

      // Right: FASTEST
      let right_x = s.i(MARGIN) + half_w as i32 + s.i(12);
      icons::draw_zap(img, right_x as u32, y as u32, TITLE_COLOR, s.factor());
      draw_text_mut(img,
                    TITLE_COLOR,
                    right_x + s.u(ICON_SZ) as i32 + s.i(4),
                    y,
                    section_title_scale,
                    font_bold,
                    "FASTEST");
      let right_y = draw_fastest_entries(img,
                                         font,
                                         font_bold,
                                         font_emoji,
                                         stats,
                                         layout,
                                         right_x,
                                         y + s.i(26),
                                         s);

      // Vertical divider
      let div_x = s.i(MARGIN) + half_w as i32;
      let div_h = (left_y.max(right_y) - y) as u32;
      draw_filled_rect_mut(img, Rect::at(div_x, y).of_size(s.u(DIVIDER_THICKNESS), div_h), BLACK);

      left_y.max(right_y) + s.i(8)
    },
    Orientation::Portrait => {
      // LONGEST full-width
      icons::draw_ruler(img, s.u(MARGIN as u32), y as u32, TITLE_COLOR, s.factor());
      draw_text_mut(img,
                    TITLE_COLOR,
                    s.i(MARGIN) + s.u(ICON_SZ) as i32 + s.i(4),
                    y,
                    section_title_scale,
                    font_bold,
                    "LONGEST");
      let y = draw_longest_entries(img,
                                   font,
                                   font_bold,
                                   font_emoji,
                                   stats,
                                   layout,
                                   s.i(MARGIN),
                                   y + s.i(26),
                                   s);

      // Separator before FASTEST
      let sep_y = y + s.i(4);
      draw_filled_rect_mut(img,
                           Rect::at(s.i(MARGIN), sep_y).of_size(content_w, s.u(DIVIDER_THICKNESS)),
                           BLACK);
      let y = sep_y + s.i(8);

      // FASTEST full-width
      icons::draw_zap(img, s.u(MARGIN as u32), y as u32, TITLE_COLOR, s.factor());
      draw_text_mut(img,
                    TITLE_COLOR,
                    s.i(MARGIN) + s.u(ICON_SZ) as i32 + s.i(4),
                    y,
                    section_title_scale,
                    font_bold,
                    "FASTEST");
      let y = draw_fastest_entries(img,
                                   font,
                                   font_bold,
                                   font_emoji,
                                   stats,
                                   layout,
                                   s.i(MARGIN),
                                   y + s.i(26),
                                   s);

      y + s.i(8)
    },
  }
}

// --- Last Activity ---

fn draw_last_activity(img: &mut RgbImage,
                      font: &FontRef,
                      font_bold: &FontRef,
                      font_emoji: &FontRef,
                      stats: &DashboardStats,
                      y_start: i32,
                      is_offline: bool,
                      config: &DisplayConfig,
                      c: Canvas,
                      s: Scale) {
  let content_w = (s.u(c.w) as i32 - 2 * s.i(MARGIN)) as u32;

  let sep_y = y_start + s.i(6);
  draw_filled_rect_mut(img,
                       Rect::at(s.i(MARGIN), sep_y).of_size(content_w, s.u(DIVIDER_THICKNESS)),
                       BLACK);

  let y = sep_y + s.i(8);

  if let Some(ref last) = stats.last_activity {
    // "LAST ACTIVITY" title
    draw_text_mut(img,
                  TITLE_COLOR,
                  s.i(MARGIN),
                  y,
                  s.px(TITLE_FONT_SZ),
                  font_bold,
                  "LAST ACTIVITY");

    // First line: sport icon + name · date
    let line1_x = s.i(MARGIN);
    icons::draw_sport_icon(img,
                           line1_x as u32,
                           (y + s.i(30)) as u32,
                           last.sport,
                           last.is_mtb,
                           BLACK,
                           s.factor());
    let line1 = format!("{}  ·  {}", truncate_str(&last.name, 30), last.date);
    draw_text_with_fallback(img,
                            MAIN_COLOR,
                            line1_x + s.u(ICON_SZ) as i32 + s.i(6),
                            y + s.i(32),
                            s.px(MAIN_FONT_SZ),
                            font_bold,
                            font_emoji,
                            &line1);

    let line2 = match last.sport {
      SportType::WeightTraining | SportType::Yoga | SportType::Pilates | SportType::Workout => {
        format!("{}  ·  {} kudos", last.moving_time_display, last.kudos)
      },
      _ => format!("{:.1}km  ·  {}  ·  {}  ·  {} kudos",
                   last.distance_km, last.pace_or_speed, last.moving_time_display, last.kudos),
    };
    draw_text_mut(img,
                  SECONDARY_COLOR,
                  line1_x + s.u(ICON_SZ) as i32 + s.i(6),
                  y + s.i(56),
                  s.px(SECONDARY_FONT_SZ),
                  font,
                  &line2);

    // Bottom reserves for battery indicator and optional elements
    let bat_w = s.u(76);
    let bat_h = if is_offline { s.u(56) } else { s.u(22) };
    let canvas_bottom = s.u(c.h) - s.u(MARGIN as u32);
    let canvas_right = s.u(c.w) - s.u(MARGIN as u32);

    let show_kudos = !config.show_totals && stats.total_kudos > 0;

    let bounds = if config.orientation == Orientation::Portrait {
      // Portrait: polyline below text, full width
      let px = s.u(MARGIN as u32);
      let py = (y + s.i(68)) as u32;
      let pw = canvas_right - px;
      // Stop before kudos text or battery indicator
      let max_bottom = if show_kudos {
        s.u(c.h) - s.u(MARGIN as u32) - s.u(24)
      } else {
        s.u(c.h) - bat_h - s.u(4)
      };
      let ph = max_bottom.saturating_sub(py);
      PolylineBounds { x: px, y: py, w: pw, h: ph }
    } else {
      // Landscape: polyline right of text
      let line1_w = measure_text_width(font_bold, s.px(MAIN_FONT_SZ), &line1) as i32;
      let line2_w = measure_text_width(font, s.px(SECONDARY_FONT_SZ), &line2) as i32;
      let text_right = line1_x + s.u(ICON_SZ) as i32 + s.i(6) + line1_w.max(line2_w);
      let px = (text_right + s.i(16)) as u32;
      let py = y as u32;
      // Trim width to avoid battery block in bottom-right corner
      let bat_x = s.u(c.w) - bat_w;
      let pw = if py + (canvas_bottom - py) > s.u(c.h) - bat_h {
        bat_x.saturating_sub(px).saturating_sub(s.u(4))
      } else {
        canvas_right.saturating_sub(px)
      };
      let ph = canvas_bottom.saturating_sub(py);
      PolylineBounds { x: px, y: py, w: pw, h: ph }
    };

    draw_polyline(img, &stats.last_activity_polyline, bounds, config.polyline_thickness, s);
  }

  // Total kudos -- bottom-left corner (only when TOTALS section is hidden)
  if !config.show_totals && stats.total_kudos > 0 {
    let kudos_text = format!("TOTAL KUDOS: {}", stats.total_kudos);
    let kudos_scale = s.px(SECONDARY_FONT_SZ);
    let kudos_y = s.u(c.h) as i32 - s.i(MARGIN);
    draw_text_mut(img, SECONDARY_COLOR, s.i(MARGIN), kudos_y, kudos_scale, font_bold, &kudos_text);
  }
}

// --- Polyline (orange route map) ---

struct PolylineBounds {
  x: u32,
  y: u32,
  w: u32,
  h: u32,
}

fn draw_polyline(img: &mut RgbImage,
                 points: &[(f64, f64)],
                 bounds: PolylineBounds,
                 thickness: u32,
                 s: Scale) {
  if points.is_empty() || bounds.w < s.u(20) || bounds.h < s.u(20) {
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
  let area_aspect = bounds.w as f64 / bounds.h as f64;

  let (draw_w, draw_h, off_x, off_y) = if route_aspect > area_aspect {
    let dw = bounds.w as f64;
    let dh = dw / route_aspect;
    (dw, dh, 0.0, (bounds.h as f64 - dh) / 2.0)
  } else {
    let dh = bounds.h as f64;
    let dw = dh * route_aspect;
    (dw, dh, (bounds.w as f64 - dw) / 2.0, 0.0)
  };

  let thickness = s.u(thickness.max(1));
  let radius = thickness as f32 / 2.0;
  let r_sq = radius * radius;
  let (img_w, img_h) = (img.width(), img.height());

  let px_points: Vec<(f32, f32)> =
    points.iter()
          .map(|&(lat, lon)| {
            let x = bounds.x as f32 + off_x as f32 + ((lon - min_lon) / lon_range * draw_w) as f32;
            let y = bounds.y as f32
                    + off_y as f32
                    + ((1.0 - (lat - min_lat) / lat_range) * draw_h) as f32;
            (x, y)
          })
          .collect();

  for window in px_points.windows(2) {
    let (x0, y0) = window[0];
    let (x1, y1) = window[1];

    let dx = x1 - x0;
    let dy = y1 - y0;
    let len = (dx * dx + dy * dy).sqrt();
    let (nx, ny) = if len > 0.0 { (-dy / len, dx / len) } else { (1.0, 0.0) };
    let half = (thickness as f32 - 1.0) / 2.0;
    for i in 0..thickness {
      let off = i as f32 - half;
      let ox = nx * off;
      let oy = ny * off;
      draw_line_segment_mut(img, (x0 + ox, y0 + oy), (x1 + ox, y1 + oy), RED);
    }
  }

  if thickness > 1 {
    for &(cx, cy) in &px_points {
      let ix_min = ((cx - radius) as i32).max(0);
      let ix_max = ((cx + radius) as i32 + 1).min(img_w as i32);
      let iy_min = ((cy - radius) as i32).max(0);
      let iy_max = ((cy + radius) as i32 + 1).min(img_h as i32);
      for iy in iy_min..iy_max {
        for ix in ix_min..ix_max {
          let dx = ix as f32 - cx;
          let dy = iy as f32 - cy;
          if dx * dx + dy * dy <= r_sq {
            img.put_pixel(ix as u32, iy as u32, ORANGE);
          }
        }
      }
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

fn truncate_str(s: &str, max_chars: usize) -> String {
  if s.chars().count() <= max_chars {
    s.to_string()
  } else {
    let truncated: String = s.chars().take(max_chars - 1).collect();
    format!("{truncated}…")
  }
}
