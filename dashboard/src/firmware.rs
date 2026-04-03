use chrono::Local;
use std::fs;
use std::path::Path;

const BOOT_CONFIG_PATH: &str = "/boot/firmware/config.txt";
const EXPECTED_CONFIG: &str = include_str!("../../dist/boot_firmware_config.txt");

/// Compare the on-disk boot firmware config with the expected version from
/// dist/ and overwrite it if they differ.
///
/// Returns `Ok(true)` if the file was updated, `Ok(false)` if it already
/// matched, and `Err` on I/O failure.
pub fn sync_boot_config() -> Result<bool, String> {
  let path = Path::new(BOOT_CONFIG_PATH);

  if !path.exists() {
    log::info!("firmware: {BOOT_CONFIG_PATH} not found (not running on RPi?) -- skipping");
    return Ok(false);
  }

  let current =
    fs::read_to_string(BOOT_CONFIG_PATH).map_err(|e| format!("Failed to read {BOOT_CONFIG_PATH}: {e}"))?;

  if current == EXPECTED_CONFIG {
    log::debug!("firmware: {BOOT_CONFIG_PATH} is up to date");
    return Ok(false);
  }

  log::info!("firmware: {BOOT_CONFIG_PATH} differs from expected -- updating");
  let timestamp = Local::now().format("%Y-%m-%d-%H-%M-%S");
  let back_up_filename = format!("/boot/firmware/config-{timestamp}.txt");
  let back_up_path = Path::new(&back_up_filename);

  fs::copy(path, back_up_path)
    .map_err(|e| format!("Failed to back-up {BOOT_CONFIG_PATH} to {back_up_path:?} :{e} (need root?)"))?;

  fs::write(path, EXPECTED_CONFIG).map_err(|e| format!("Failed to write {BOOT_CONFIG_PATH}: {e}"))?;

  log::info!("firmware: {BOOT_CONFIG_PATH} updated successfully. Back up at : {back_up_filename}");
  Ok(true)
}
