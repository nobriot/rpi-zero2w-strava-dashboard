use chrono::Utc;
use std::process::Command;

/// Try to power off the Pi and schedule a wake-up via the DS3231 RTC.
///
/// Requires the DS3231 INT/SQW pin to be wired to GPIO4, with the
/// `gpio-shutdown,gpio_pin=4` device-tree overlay enabled so the Pi
/// wakes from halt on that pin.
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

/// Check whether any SSH (pseudo-terminal) sessions are currently active.
///
/// Runs `who` and looks for `pts/` entries. Returns `false` on failure
/// (conservative: don't block shutdown if we can't check).
pub fn has_ssh_sessions() -> bool {
  let output = match Command::new("who").output() {
    Ok(o) => o,
    Err(e) => {
      log::debug!("Failed to run `who`: {e}");
      return false;
    },
  };

  let stdout = String::from_utf8_lossy(&output.stdout);
  let has_sessions = stdout.lines().any(|line| line.contains("pts/"));
  if has_sessions {
    log::info!("Active SSH session(s) detected");
  }
  has_sessions
}

/// Read the current battery status from the INA219 UPS monitor.
///
/// Returns `None` if the sensor is unavailable (e.g. no UPS board, dev
/// machine). This is non-fatal — the caller uses `None` to mean "on
/// external power / no battery".
pub fn read_battery() -> Option<display::ina219::BatteryStatus> {
  match display::ina219::Ina219::new().and_then(|mut ina| ina.read_status()) {
    Ok(status) => {
      log::info!("Battery: {}% ({:.2}V, {})",
                 status.percentage,
                 status.voltage,
                 if status.is_charging { "charging" } else { "discharging" });
      Some(status)
    },
    Err(e) => {
      log::debug!("Battery monitor unavailable: {e}");
      None
    },
  }
}
