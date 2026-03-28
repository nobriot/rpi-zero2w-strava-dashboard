use chrono::Utc;
use std::process::Command;

/// Disable HDMI output to save power (~20-30mA). The e-paper display
/// does not use HDMI, so this is always safe.
pub fn disable_hdmi() {
  match Command::new("tvservice").arg("-o").output() {
    Ok(output) if output.status.success() => log::info!("HDMI disabled"),
    Ok(output) => {
      let stderr = String::from_utf8_lossy(&output.stderr);
      log::info!("tvservice -o: {}", stderr.trim());
    },
    Err(_) => log::info!("tvservice not available (non-fatal)"),
  }
}

/// Enter low-power mode: disable WiFi radio to save ~30-40mA during sleep.
#[expect(dead_code,
         reason = "kept for future use when WiFi sleep is re-enabled")]
pub fn enter_low_power() {
  log::info!("Entering low-power sleep (disabling WiFi)");
  let _ = Command::new("sudo").args(["rfkill", "block", "wifi"]).output();
}

/// Exit low-power mode: re-enable WiFi and wait for reconnection.
pub fn exit_low_power() {
  log::info!("Exiting low-power sleep (re-enabling WiFi)");
  let _ = Command::new("sudo").args(["rfkill", "unblock", "wifi"]).output();
  // Give WiFi time to reassociate and get an IP
  std::thread::sleep(std::time::Duration::from_secs(10));
}

/// Try to power off the Pi and schedule a wake-up via the DS3231 RTC.
///
/// Requires the DS3231 INT/SQW pin to be wired to GPIO3 (pin 5) — the Pi's
/// only wake-from-poweroff pin. On the Waveshare PhotoPainter board this
/// connection does not exist by default.
///
/// Returns `true` if rtcwake initiated a poweroff (caller should break the
/// loop). Returns `false` if unavailable — caller should fall back to
/// low-power sleep.
pub fn try_rtcwake_shutdown(sleep_secs: u64) -> bool {
  if !std::path::Path::new("/dev/rtc0").exists() {
    log::info!("rtcwake: /dev/rtc0 not available, using low-power sleep");
    return false;
  }

  let wake_epoch = Utc::now().timestamp() as u64 + sleep_secs;
  log::info!("rtcwake: attempting poweroff with wake at epoch {wake_epoch} (in {sleep_secs}s)");

  let rtcwake = Command::new("sudo").args(["rtcwake", "-m", "off", "-t", &wake_epoch.to_string()])
                                    .output();

  match rtcwake {
    Ok(output) if output.status.success() => {
      log::info!("rtcwake initiated poweroff");
      true
    },
    Ok(output) => {
      let stderr = String::from_utf8_lossy(&output.stderr);
      log::info!("rtcwake unavailable ({}) — using low-power sleep", stderr.trim());
      false
    },
    Err(e) => {
      log::info!("rtcwake not found ({e}) — using low-power sleep");
      false
    },
  }
}
