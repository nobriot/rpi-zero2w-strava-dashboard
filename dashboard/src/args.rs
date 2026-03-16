use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = env!("CARGO_PKG_NAME"), max_term_width = 80)]
#[command(about = "rpi-zero2w-strava-dash")]
#[command(version)]
pub struct Args {
  /// Force a Strava auth flow, to get a token that has read scope
  /// to all activities
  #[arg(short, long)]
  pub auth: bool,

  /// Path to a config file (default:
  /// ~/.config/rpi-zero2w-strava-dash/config.toml)
  #[arg(short, long, value_name = "FILE")]
  pub config: Option<PathBuf>,

  /// Run a single cycle (fetch → render → display) and exit
  #[arg(long)]
  pub once: bool,

  /// Save the rendered dashboard as a PNG file (for testing without e-paper)
  #[arg(long, value_name = "PATH")]
  pub save_png: Option<String>,

  /// Clear all cached data (athlete, stats, activities, avatar)
  #[arg(long)]
  pub clear_cache: bool,

  /// Show all sports (run/ride/swim) even if no activities exist (demo mode)
  #[arg(long)]
  pub show_all_sports: bool,

  /// Resolution scale factor for PNG export (default: 1, use 2 for hi-res)
  #[arg(long, default_value = "1", value_name = "N")]
  pub scale: u32,
}
