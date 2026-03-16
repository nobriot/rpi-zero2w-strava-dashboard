use image::Rgb;

/// The 6 colors supported by the Waveshare 7.3" ACeP e-Paper display.
/// Wire format values (4-bit per pixel).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EpdColor {
  Black = 0,
  White = 1,
  Green = 2,
  Blue = 3,
  Red = 4,
  Yellow = 5,
}

impl EpdColor {
  pub const ALL: [EpdColor; 6] = [EpdColor::Black,
                                  EpdColor::White,
                                  EpdColor::Green,
                                  EpdColor::Blue,
                                  EpdColor::Red,
                                  EpdColor::Yellow];

  /// Approximate RGB value for this display color.
  pub fn to_rgb(self) -> Rgb<u8> {
    match self {
      EpdColor::Black => Rgb([0, 0, 0]),
      EpdColor::White => Rgb([255, 255, 255]),
      EpdColor::Green => Rgb([0, 128, 0]),
      EpdColor::Blue => Rgb([0, 0, 128]),
      EpdColor::Red => Rgb([200, 0, 0]),
      EpdColor::Yellow => Rgb([220, 200, 0]),
    }
  }
}

/// Find the nearest display color for an RGB pixel using squared Euclidean
/// distance.
pub fn nearest_color(pixel: Rgb<u8>) -> EpdColor {
  EpdColor::ALL.iter()
               .min_by_key(|c| {
                 let target = c.to_rgb();
                 let dr = pixel[0] as i32 - target[0] as i32;
                 let dg = pixel[1] as i32 - target[1] as i32;
                 let db = pixel[2] as i32 - target[2] as i32;
                 dr * dr + dg * dg + db * db
               })
               .copied()
               .unwrap()
}

/// Quantize an RGB image to the 6-color palette and pack into the EPD wire
/// format. Returns a buffer of WIDTH*HEIGHT/2 bytes (2 pixels per byte, high
/// nibble first).
pub fn quantize_to_epd_buffer(img: &image::RgbImage) -> Vec<u8> {
  let (w, h) = img.dimensions();
  let mut buf = Vec::with_capacity((w as usize * h as usize) / 2);

  for y in 0..h {
    let mut x = 0u32;
    while x < w {
      let c1 = nearest_color(*img.get_pixel(x, y)) as u8;
      let c2 = if x + 1 < w {
        nearest_color(*img.get_pixel(x + 1, y)) as u8
      } else {
        EpdColor::White as u8
      };
      buf.push((c1 << 4) | c2);
      x += 2;
    }
  }

  buf
}
