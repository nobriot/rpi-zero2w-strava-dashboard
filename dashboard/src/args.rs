use crate::errors::DashError;
use clap::builder::styling;
use clap::{CommandFactory, FromArgMatches, Parser};
use std::path::PathBuf;

const STYLES: styling::Styles =
  styling::Styles::styled().header(styling::AnsiColor::Green.on_default().bold())
                           .usage(styling::AnsiColor::Green.on_default().bold())
                           .literal(styling::AnsiColor::Blue.on_default().bold())
                           .placeholder(styling::AnsiColor::Cyan.on_default());

#[derive(Parser, Debug)]
#[command(name = env!("CARGO_BIN_NAME"), max_term_width = 80)]
#[command(about = env!("CARGO_PKG_DESCRIPTION"))]
#[command(version)]
pub struct Args {
  /// Force a Strava auth flow, to get a token that has read scope
  /// to all activities
  #[arg(short, long)]
  pub auth: bool,

  /// Path to a config file (default: ~/.config/<app>/config.toml)
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

  /// Sync /boot/firmware/config.txt with the expected version at startup
  #[arg(long)]
  pub sync_firmware: bool,

  /// Resolution scale factor for PNG export (default: 1, use 2 for hi-res)
  #[arg(long, default_value = "1", value_name = "N")]
  pub scale: u32,

  /// Polyline thickness in pixels (overrides config.toml)
  #[arg(long, value_name = "PX")]
  pub polyline_thickness: Option<u32>,
}

impl Args {
  pub fn try_parse() -> std::result::Result<Self, DashError> {
    let mut matches = Args::command().styles(STYLES).term_width(80).get_matches();
    let args =
      Args::from_arg_matches_mut(&mut matches).map_err(|e| DashError::Argument(e.to_string()))?;

    Ok(args)
  }
}
