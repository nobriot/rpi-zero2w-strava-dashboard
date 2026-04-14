use crate::errors::{DashError, Result};
use std::path::Path;

pub struct RenderRequest<'a> {
  pub stats:       &'a common::DashboardStats,
  pub battery:     Option<&'a common::BatteryStatus>,
  pub avatar:      Option<&'a [u8]>,
  pub is_offline:  bool,
  pub display_cfg: &'a display::config::DisplayConfig,
  pub scale:       u32,
  pub save_png:    Option<&'a Path>,
}

/// Render the dashboard and present it: save PNG if requested, push to the
/// e-paper display if available, or fall back to a preview PNG on disk.
pub fn present(req: RenderRequest<'_>) -> Result<()> {
  let preview_scale = display::renderer::Scale::new(req.scale);
  let preview = display::renderer::render_dashboard(req.stats,
                                                    req.battery,
                                                    req.display_cfg,
                                                    req.avatar,
                                                    req.is_offline,
                                                    preview_scale);

  if let Some(path) = req.save_png {
    preview.save(path).map_err(render_err)?;
    log::info!("Dashboard saved to {}", path.display());
  }

  // Try to push to e-paper. Always render at 2x and downsample -- area
  // averaging darkens anti-aliased font edges so they survive 6-color
  // quantization as black instead of vanishing to white.
  match display::epd7in3e::Epd7in3e::new() {
    Ok(mut epd) => {
      let ss_scale = display::renderer::Scale::new(2);
      let epd_img = display::renderer::render_dashboard(req.stats,
                                                        req.battery,
                                                        req.display_cfg,
                                                        req.avatar,
                                                        req.is_offline,
                                                        ss_scale);
      let epd_img = if req.display_cfg.flip {
        image::imageops::rotate180(&epd_img)
      } else {
        epd_img
      };
      let buf = display::palette::quantize_supersampled_to_epd_buffer(&epd_img, 800, 480);
      epd.display_image(&buf)?;
      epd.sleep()?;
      log::info!("E-paper display updated");
    },
    Err(e) => {
      log::info!("E-paper display not available: {e}");
      if req.save_png.is_none() {
        let fallback = Path::new("dashboard_preview.png");
        preview.save(fallback).map_err(render_err)?;
        log::info!("Dashboard saved to {} (no display available)", fallback.display());
      }
    },
  }

  Ok(())
}

fn render_err(e: image::ImageError) -> DashError {
  DashError::Display(display::errors::DisplayError::Render(e.to_string()))
}
