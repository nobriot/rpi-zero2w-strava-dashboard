use image::{Rgb, RgbImage, RgbaImage};

/// Icon size in logical pixels (each icon fits in a SIZE×SIZE box).
pub const SIZE: u32 = 20;

const ICON_RUN_SVG: &str = include_str!("../assets/icon_run.svg");
const ICON_RIDE_SVG: &str = include_str!("../assets/icon_bike.svg");
const ICON_MTB_SVG: &str = include_str!("../assets/icon_mtb.svg");
const ICON_SWIM_SVG: &str = include_str!("../assets/icon_swim.svg");
const ICON_WEIGHT_SVG: &str = include_str!("../assets/icon_weight.svg");
const ICON_YOGA_SVG: &str = include_str!("../assets/icon_yoga.svg");
const ICON_PILATES_SVG: &str = include_str!("../assets/icon_pilates.svg");
const ICON_WORKOUT_SVG: &str = include_str!("../assets/icon_workout.svg");

const ICON_RULER_SVG: &str = include_str!("../assets/icon_ruler.svg");
const ICON_BAR_CHART_SVG: &str = include_str!("../assets/icon_bar_chart.svg");
const ICON_ZAP_SVG: &str = include_str!("../assets/icon_zap.svg");

/// Rasterize an SVG string to an RGBA image at the given pixel size, with the
/// specified fill color.
fn rasterize_svg(svg_str: &str, size: u32, color: Rgb<u8>) -> Option<RgbaImage> {
  let hex = format!("#{:02X}{:02X}{:02X}", color[0], color[1], color[2]);
  let colored = svg_str.replace("currentColor", &hex)
                       .replace("fill=\"\"", &format!("fill=\"{hex}\""))
                       .replace("stroke=\"\"", &format!("stroke=\"{hex}\""));

  let tree = resvg::usvg::Tree::from_str(&colored, &resvg::usvg::Options::default()).ok()?;
  let svg_size = tree.size();
  let sx = size as f32 / svg_size.width();
  let sy = size as f32 / svg_size.height();

  let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size)?;
  resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(sx, sy), &mut pixmap.as_mut());

  // Convert from premultiplied to straight alpha
  let mut data = pixmap.take();
  for chunk in data.chunks_exact_mut(4) {
    let a = chunk[3] as f32 / 255.0;
    if a > 0.0 {
      chunk[0] = (chunk[0] as f32 / a).min(255.0) as u8;
      chunk[1] = (chunk[1] as f32 / a).min(255.0) as u8;
      chunk[2] = (chunk[2] as f32 / a).min(255.0) as u8;
    }
  }

  RgbaImage::from_raw(size, size, data)
}

/// Overlay an SVG icon onto the image at (x, y), rasterized at
/// SIZE * scale pixels with the given color.
fn draw_svg_icon(img: &mut RgbImage, x: u32, y: u32, svg: &str, color: Rgb<u8>, scale: u32) {
  let target_sz = SIZE * scale;
  let icon = match rasterize_svg(svg, target_sz, color) {
    Some(i) => i,
    None => {
      log::warn!("Failed to rasterize SVG icon");
      return;
    },
  };
  composite_rgba(img, x, y, &icon, color);
}

/// Alpha-composite an RGBA icon onto the RGB image, tinting with the given
/// color.
fn composite_rgba(img: &mut RgbImage, x: u32, y: u32, icon: &RgbaImage, color: Rgb<u8>) {
  for py in 0..icon.height() {
    for px in 0..icon.width() {
      let p = icon.get_pixel(px, py);
      let alpha = p[3] as f32 / 255.0;
      if alpha < 0.1 {
        continue;
      }
      let ix = x + px;
      let iy = y + py;
      if ix < img.width() && iy < img.height() {
        let bg = img.get_pixel(ix, iy);
        let r = (color[0] as f32 * alpha + bg[0] as f32 * (1.0 - alpha)) as u8;
        let g = (color[1] as f32 * alpha + bg[1] as f32 * (1.0 - alpha)) as u8;
        let b = (color[2] as f32 * alpha + bg[2] as f32 * (1.0 - alpha)) as u8;
        img.put_pixel(ix, iy, Rgb([r, g, b]));
      }
    }
  }
}

pub fn draw_runner(img: &mut RgbImage, x: u32, y: u32, color: Rgb<u8>, scale: u32) {
  draw_svg_icon(img, x, y, ICON_RUN_SVG, color, scale);
}

pub fn draw_cyclist(img: &mut RgbImage, x: u32, y: u32, color: Rgb<u8>, scale: u32) {
  draw_svg_icon(img, x, y, ICON_RIDE_SVG, color, scale);
}

pub fn draw_mtb(img: &mut RgbImage, x: u32, y: u32, color: Rgb<u8>, scale: u32) {
  draw_svg_icon(img, x, y, ICON_MTB_SVG, color, scale);
}

pub fn draw_swimmer(img: &mut RgbImage, x: u32, y: u32, color: Rgb<u8>, scale: u32) {
  draw_svg_icon(img, x, y, ICON_SWIM_SVG, color, scale);
}

pub fn draw_weight(img: &mut RgbImage, x: u32, y: u32, color: Rgb<u8>, scale: u32) {
  draw_svg_icon(img, x, y, ICON_WEIGHT_SVG, color, scale);
}

pub fn draw_yoga(img: &mut RgbImage, x: u32, y: u32, color: Rgb<u8>, scale: u32) {
  draw_svg_icon(img, x, y, ICON_YOGA_SVG, color, scale);
}

pub fn draw_pilates(img: &mut RgbImage, x: u32, y: u32, color: Rgb<u8>, scale: u32) {
  draw_svg_icon(img, x, y, ICON_PILATES_SVG, color, scale);
}

pub fn draw_workout(img: &mut RgbImage, x: u32, y: u32, color: Rgb<u8>, scale: u32) {
  draw_svg_icon(img, x, y, ICON_WORKOUT_SVG, color, scale);
}

pub fn draw_ruler(img: &mut RgbImage, x: u32, y: u32, color: Rgb<u8>, scale: u32) {
  draw_svg_icon(img, x, y, ICON_RULER_SVG, color, scale);
}

pub fn draw_bar_chart(img: &mut RgbImage, x: u32, y: u32, color: Rgb<u8>, scale: u32) {
  draw_svg_icon(img, x, y, ICON_BAR_CHART_SVG, color, scale);
}

pub fn draw_zap(img: &mut RgbImage, x: u32, y: u32, color: Rgb<u8>, scale: u32) {
  draw_svg_icon(img, x, y, ICON_ZAP_SVG, color, scale);
}

/// Draw the sport-appropriate icon.
/// When `is_mtb` is true and the sport is Ride, use the mountain bike icon.
pub fn draw_sport_icon(img: &mut RgbImage,
                       x: u32,
                       y: u32,
                       sport: common::SportType,
                       is_mtb: bool,
                       color: Rgb<u8>,
                       scale: u32) {
  match sport {
    common::SportType::Run => draw_runner(img, x, y, color, scale),
    common::SportType::Ride if is_mtb => draw_mtb(img, x, y, color, scale),
    common::SportType::Ride => draw_cyclist(img, x, y, color, scale),
    common::SportType::Swim => draw_swimmer(img, x, y, color, scale),
    common::SportType::WeightTraining => draw_weight(img, x, y, color, scale),
    common::SportType::Yoga => draw_yoga(img, x, y, color, scale),
    common::SportType::Pilates => draw_pilates(img, x, y, color, scale),
    common::SportType::Workout => draw_workout(img, x, y, color, scale),
  }
}

/// Draw a battery outline at (x, y) with fill level 0.0–1.0.
pub fn draw_battery(img: &mut RgbImage,
                    x: u32,
                    y: u32,
                    outline_color: Rgb<u8>,
                    fill_color: Rgb<u8>,
                    fill: f32,
                    scale: u32) {
  let outline: &[(u32, u32)] = &[(1, 2),
                                 (2, 2),
                                 (3, 2),
                                 (4, 2),
                                 (5, 2),
                                 (6, 2),
                                 (7, 2),
                                 (8, 2),
                                 (9, 2),
                                 (10, 2),
                                 (11, 2),
                                 (1, 9),
                                 (2, 9),
                                 (3, 9),
                                 (4, 9),
                                 (5, 9),
                                 (6, 9),
                                 (7, 9),
                                 (8, 9),
                                 (9, 9),
                                 (10, 9),
                                 (11, 9),
                                 (1, 3),
                                 (1, 4),
                                 (1, 5),
                                 (1, 6),
                                 (1, 7),
                                 (1, 8),
                                 (11, 3),
                                 (11, 4),
                                 (11, 5),
                                 (11, 6),
                                 (11, 7),
                                 (11, 8),
                                 (12, 4),
                                 (12, 5),
                                 (12, 6),
                                 (12, 7),
                                 (13, 4),
                                 (13, 5),
                                 (13, 6),
                                 (13, 7)];
  for &(px, py) in outline {
    for dx in 0..scale {
      for dy in 0..scale {
        let ix = x + px * scale + dx;
        let iy = y + py * scale + dy;
        if ix < img.width() && iy < img.height() {
          img.put_pixel(ix, iy, outline_color);
        }
      }
    }
  }
  let fill_cols = ((9.0 * fill.clamp(0.0, 1.0)) as u32).min(9);
  for col in 0..fill_cols {
    for row in 3..9u32 {
      for dx in 0..scale {
        for dy in 0..scale {
          let ix = x + (2 + col) * scale + dx;
          let iy = y + row * scale + dy;
          if ix < img.width() && iy < img.height() {
            img.put_pixel(ix, iy, fill_color);
          }
        }
      }
    }
  }
}
