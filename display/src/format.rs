//! Formatters used across the renderer.
//!
//! These helpers exist to keep adjacent rows visually aligned: each one
//! emits a fixed-width string (with leading spaces in place of leading
//! zero components) so the same column always contains the same kind of
//! digit.

/// Format a single-activity duration: "Hh MMm SSs" (e.g. "1h 23m 45s").
/// When the hours component is zero the leading "0h " is blanked out with
/// three spaces ("   23m 45s") so adjacent rows still align by the minute
/// column. Hours are not zero-padded; single-digit hours dominate.
///
/// Use this when the duration is part of a single rendered text run
/// (a one-shot `format!` line). For tabular layouts where the minute
/// column must line up across rows, use [`fmt_short_hours_prefix`] +
/// [`fmt_short_min_sec`] and draw the two sub-columns separately --
/// proportional fonts collapse leading spaces, so the blanked-out form
/// alone is not enough for pixel alignment.
pub fn fmt_duration_short(secs: u32) -> String {
  let hours = secs / 3600;
  let minutes = (secs % 3600) / 60;
  let seconds = secs % 60;
  if hours == 0 {
    format!("   {minutes:02}m {seconds:02}s")
  } else {
    format!("{hours}h {minutes:02}m {seconds:02}s")
  }
}

/// Hours sub-component of a short duration: "Xh " (with trailing space)
/// when hours > 0, empty otherwise. Pair with [`fmt_short_min_sec`] for
/// column-aligned tables; the empty string lets you skip drawing
/// entirely on rows without an hours component.
pub fn fmt_short_hours_prefix(secs: u32) -> String {
  let hours = secs / 3600;
  if hours == 0 { String::new() } else { format!("{hours}h ") }
}

/// Minute/second sub-component of a short duration: "MMm SSs", always
/// emitted with two-digit padding so the column has a stable shape.
pub fn fmt_short_min_sec(secs: u32) -> String {
  let minutes = (secs % 3600) / 60;
  let seconds = secs % 60;
  format!("{minutes:02}m {seconds:02}s")
}

/// Format an aggregate / goal-bar duration: "Dd HHh MMm" (e.g.
/// "12d 03h 25m"). When the days component is zero the leading "0d " is
/// blanked out with three spaces ("   05h 23m") so adjacent rows still
/// align by the hour column.
pub fn fmt_duration_long(secs: u32) -> String {
  let total_minutes = secs / 60;
  let days = total_minutes / 1440;
  let hours = (total_minutes % 1440) / 60;
  let minutes = total_minutes % 60;
  if days == 0 {
    format!("   {hours:02}h {minutes:02}m")
  } else {
    format!("{days}d {hours:02}h {minutes:02}m")
  }
}

/// Format a distance as up to 3 digits + 1 decimal followed by "km",
/// right-aligned in a 5-wide numeric field with leading spaces:
/// "  1.0km", " 20.3km", "120.4km". Distances >= 1000km get a wider
/// numeric field rather than overflowing.
pub fn fmt_distance_km(km: f64) -> String {
  format!("{km:>5.1}km")
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn short_blanks_zero_hours() {
    assert_eq!(fmt_duration_short(0), "   00m 00s");
    assert_eq!(fmt_duration_short(45), "   00m 45s");
    assert_eq!(fmt_duration_short(60 + 5), "   01m 05s");
    assert_eq!(fmt_duration_short(3600 + 23 * 60 + 45), "1h 23m 45s");
    assert_eq!(fmt_duration_short(12 * 3600 + 5 * 60 + 9), "12h 05m 09s");
  }

  #[test]
  fn long_blanks_zero_days() {
    assert_eq!(fmt_duration_long(0), "   00h 00m");
    assert_eq!(fmt_duration_long(3600 * 5 + 23 * 60), "   05h 23m");
    assert_eq!(fmt_duration_long(86_400 + 3600 * 3 + 25 * 60), "1d 03h 25m");
    assert_eq!(fmt_duration_long(86_400 * 12 + 3600 * 5 + 23 * 60), "12d 05h 23m");
  }

  #[test]
  fn short_split_components() {
    assert_eq!(fmt_short_hours_prefix(0), "");
    assert_eq!(fmt_short_hours_prefix(45), "");
    assert_eq!(fmt_short_hours_prefix(3600 + 5), "1h ");
    assert_eq!(fmt_short_hours_prefix(12 * 3600 + 5), "12h ");

    assert_eq!(fmt_short_min_sec(0), "00m 00s");
    assert_eq!(fmt_short_min_sec(45), "00m 45s");
    assert_eq!(fmt_short_min_sec(3600 + 23 * 60 + 45), "23m 45s");
    assert_eq!(fmt_short_min_sec(12 * 3600 + 5 * 60 + 9), "05m 09s");
  }

  #[test]
  fn distance_padded_to_five_digits() {
    assert_eq!(fmt_distance_km(1.0), "  1.0km");
    assert_eq!(fmt_distance_km(20.3), " 20.3km");
    assert_eq!(fmt_distance_km(120.4), "120.4km");
    assert_eq!(fmt_distance_km(0.0), "  0.0km");
  }
}
