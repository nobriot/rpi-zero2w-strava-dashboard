mod args;
mod auth;
mod config;
mod cycle;
mod errors;
mod firmware;
mod heartbeat;
mod ina219;
mod logging;
mod net;
mod power;
mod render;
mod runloop;
mod schedule;
mod stats;

use args::Args;
use config::Config;
use errors::{DashError, Result};

static PROGRAM_NAME: &str = env!("CARGO_PKG_NAME");

fn main() {
  if let Err(e) = run() {
    eprintln!("{PROGRAM_NAME} - error: {e:?}");
    std::process::exit(1);
  }
}

fn run() -> Result<()> {
  let args = Args::try_parse()?;
  logging::setup(args.log_file.as_deref());

  if args.auth {
    return auth::run(args.config.as_ref());
  }

  if args.clear_cache {
    strava::cache::Cache::new().clear().map_err(DashError::Config)?;
    eprintln!("Cache cleared.");
  }

  let config = match args.config.as_ref() {
                 Some(path) => Config::load_from(path),
                 None => Config::load(),
               }.map_err(DashError::Config)?;
  log::info!("Config loaded successfully");

  if args.sync_firmware || config.power.sync_firmware {
    match firmware::sync_boot_config() {
      Ok(true) => log::info!("Boot firmware config updated"),
      Ok(false) => {},
      Err(e) => log::warn!("Failed to sync boot firmware config: {e}"),
    }
  }

  runloop::run(config, args)
}
