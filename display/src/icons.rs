use image::{Rgb, RgbImage};

/// Icon size in pixels (each icon fits in a SIZE×SIZE box).
pub const SIZE: u32 = 24;

const ICON_RUN: &[u8] = include_bytes!("../assets/icon_run.png");
const ICON_RIDE: &[u8] = include_bytes!("../assets/icon_ride.png");
const ICON_SWIM: &[u8] = include_bytes!("../assets/icon_swim.png");
const ICON_LIGHTNING: &[u8] = include_bytes!("../assets/icon_lightning.png");
const ICON_RULER: &[u8] = include_bytes!("../assets/icon_ruler.png");
const ICON_KUDOS: &[u8] = include_bytes!("../assets/icon_kudos.png");
const ICON_BAR_CHART: &[u8] = include_bytes!("../assets/icon_bar_chart.png");

/// Overlay a pre-rendered PNG icon onto the image at (x, y), tinting non-transparent
/// pixels with the given color. When scale > 1, the icon is resized up.
fn draw_icon(img: &mut RgbImage, x: u32, y: u32, icon_bytes: &[u8], color: Rgb<u8>, scale: u32) {
    let raw = match image::load_from_memory(icon_bytes) {
        Ok(i) => i.to_rgba8(),
        Err(e) => {
            log::warn!("Failed to decode icon: {e}");
            return;
        }
    };

    let icon = if scale > 1 {
        image::imageops::resize(
            &raw,
            SIZE * scale,
            SIZE * scale,
            image::imageops::FilterType::Triangle,
        )
    } else {
        raw
    };

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
    draw_icon(img, x, y, ICON_RUN, color, scale);
}

pub fn draw_cyclist(img: &mut RgbImage, x: u32, y: u32, color: Rgb<u8>, scale: u32) {
    draw_icon(img, x, y, ICON_RIDE, color, scale);
}

pub fn draw_swimmer(img: &mut RgbImage, x: u32, y: u32, color: Rgb<u8>, scale: u32) {
    draw_icon(img, x, y, ICON_SWIM, color, scale);
}

pub fn draw_lightning(img: &mut RgbImage, x: u32, y: u32, color: Rgb<u8>, scale: u32) {
    draw_icon(img, x, y, ICON_LIGHTNING, color, scale);
}

pub fn draw_ruler(img: &mut RgbImage, x: u32, y: u32, color: Rgb<u8>, scale: u32) {
    draw_icon(img, x, y, ICON_RULER, color, scale);
}

pub fn draw_thumbs_up(img: &mut RgbImage, x: u32, y: u32, color: Rgb<u8>, scale: u32) {
    draw_icon(img, x, y, ICON_KUDOS, color, scale);
}

pub fn draw_bar_chart(img: &mut RgbImage, x: u32, y: u32, color: Rgb<u8>, scale: u32) {
    draw_icon(img, x, y, ICON_BAR_CHART, color, scale);
}

/// Draw the sport-appropriate icon.
pub fn draw_sport_icon(
    img: &mut RgbImage,
    x: u32,
    y: u32,
    sport: strava::types::SportType,
    color: Rgb<u8>,
    scale: u32,
) {
    match sport {
        strava::types::SportType::Run => draw_runner(img, x, y, color, scale),
        strava::types::SportType::Ride => draw_cyclist(img, x, y, color, scale),
        strava::types::SportType::Swim => draw_swimmer(img, x, y, color, scale),
    }
}

/// Draw a 12×12 checkered flag at (x, y).
pub fn draw_checkered_flag(img: &mut RgbImage, x: u32, y: u32, color: Rgb<u8>, scale: u32) {
    // Flagpole
    for row in 0..12 {
        for dx in 0..scale {
            for dy in 0..scale {
                let ix = x + dx;
                let iy = y + row * scale + dy;
                if ix < img.width() && iy < img.height() {
                    img.put_pixel(ix, iy, color);
                }
            }
        }
    }
    // 8×6 checkerboard flag
    for row in 0..6u32 {
        for col in 0..8u32 {
            let checker = ((row / 2) + (col / 2)) % 2 == 0;
            if checker {
                for dx in 0..scale {
                    for dy in 0..scale {
                        let ix = x + (2 + col) * scale + dx;
                        let iy = y + row * scale + dy;
                        if ix < img.width() && iy < img.height() {
                            img.put_pixel(ix, iy, color);
                        }
                    }
                }
            }
        }
    }
}

/// Draw a battery outline at (x, y) with fill level 0.0–1.0.
pub fn draw_battery(img: &mut RgbImage, x: u32, y: u32, color: Rgb<u8>, fill: f32, scale: u32) {
    let outline: &[(u32, u32)] = &[
        (1, 2),
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
        (13, 7),
    ];
    for &(px, py) in outline {
        for dx in 0..scale {
            for dy in 0..scale {
                let ix = x + px * scale + dx;
                let iy = y + py * scale + dy;
                if ix < img.width() && iy < img.height() {
                    img.put_pixel(ix, iy, color);
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
                        img.put_pixel(ix, iy, color);
                    }
                }
            }
        }
    }
}
