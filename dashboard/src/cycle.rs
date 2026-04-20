use crate::args::Args;
use crate::config::Config;
use crate::errors::{DashError, Result};
use crate::power::{self, PowerManager};
use crate::render::{self, RenderRequest};
use crate::stats;
use std::path::Path;
use strava::errors::StravaError;

/// Run one dashboard cycle with error recovery (OAuth re-auth, network retry).
pub fn run(config: &mut Config,
           client: &mut Option<strava::client::Client>,
           args: &Args,
           power_mgr: &mut PowerManager)
           -> Result<()> {
  match run_once(config, client, args) {
    Ok(()) => Ok(()),
    Err(DashError::Strava(StravaError::Unauthorized)) => {
      // Discard the old client -- its token is invalid
      *client = None;
      recover_auth(config, client, args, power_mgr)
    },
    Err(DashError::Strava(StravaError::NetworkUnavailable(ref msg))) => {
      log::warn!("Network unavailable: {msg} -- will retry next cycle");
      eprintln!("Network unavailable -- will retry next cycle");
      Ok(())
    },
    Err(e) => {
      log::error!("Error during cycle: {e:?}");
      eprintln!("Error during cycle: {e:?}");
      Ok(())
    },
  }
}

fn run_once(config: &mut Config,
            client: &mut Option<strava::client::Client>,
            args: &Args)
            -> Result<()> {
  let fetched = stats::fetch(config, client, args.show_all_sports)?;
  let battery = power::read_battery();

  let polyline_thickness = args.polyline_thickness.unwrap_or(config.display.polyline_thickness);
  let display_cfg = display::config::DisplayConfig { polyline_thickness,
                                                     ..config.display.clone() };

  render::present(RenderRequest { stats:       &fetched.stats,
                                  battery:     battery.as_ref(),
                                  avatar:      fetched.avatar.as_deref(),
                                  is_offline:  fetched.is_offline,
                                  display_cfg: &display_cfg,
                                  scale:       args.scale,
                                  save_png:    args.save_png.as_deref().map(Path::new),
                                  skip_epaper: args.kiosk, })?;

  fetched.stats.print_summary();
  Ok(())
}

fn recover_auth(config: &mut Config,
                client: &mut Option<strava::client::Client>,
                args: &Args,
                power_mgr: &mut PowerManager)
                -> Result<()> {
  log::warn!("Unauthorized after auto-refresh -- attempting full OAuth re-authorization");
  eprintln!("\nRefresh token invalid. Starting OAuth authorization flow...");

  let token_response = strava::oauth::run_auth_flow(&config.strava)?;
  config.strava.set_refresh_token(token_response.refresh_token);
  config.save().map_err(DashError::Config)?;

  power_mgr.enable_wifi();
  if let Err(e) = run_once(config, client, args) {
    eprintln!("Error after re-authorization: {e:?}");
  }
  Ok(())
}
