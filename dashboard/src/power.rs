use chrono::Utc;
use std::fs;
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

/// Disable non-essential peripherals to reduce power draw during sleep.
/// Turns off HDMI, Bluetooth, and USB/LAN. Safe to call on dev machines
/// (failures are logged and ignored).
pub fn set_peripherals_low_power() {
  log::info!("Disabling non-essential peripherals for low-power sleep");
  set_hdmi_status(Status::Disabled);
  set_bluetooth_status(Status::Disabled);
  set_usb_status(Status::Disabled);
}

/// Re-enable peripherals before the next active cycle.
pub fn set_peripherals_normal() {
  log::info!("Re-enabling peripherals");
  set_usb_status(Status::Enabled);
  set_bluetooth_status(Status::Enabled);
  set_hdmi_status(Status::Enabled);
}

/// Whether a peripheral is currently enabled or disabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
  Enabled,
  Disabled,
}

// ── USB / LAN ───────────────────────────────────────────────────────────────
// The RPi Zero 2W exposes USB and LAN on a single hub at port 1-1.
// Unbinding it from the kernel driver saves ~100mA.
// See: https://raspberrypi-guide.github.io/electronics/power-consumption-tricks

const USB_BIND: &str = "/sys/bus/usb/drivers/usb/bind";
const USB_UNBIND: &str = "/sys/bus/usb/drivers/usb/unbind";
const USB_PORT: &str = "1-1";

/// Read whether the USB/LAN hub is currently bound to its kernel driver.
pub fn get_usb_status() -> Status {
  // If the port directory exists under the driver, the device is bound
  if std::path::Path::new(&format!("/sys/bus/usb/drivers/usb/{USB_PORT}")).exists() {
    Status::Enabled
  } else {
    Status::Disabled
  }
}

/// Enable or disable the USB/LAN hub. Returns the resulting status.
pub fn set_usb_status(status: Status) -> Status {
  let path = match status {
    Status::Enabled => USB_BIND,
    Status::Disabled => USB_UNBIND,
  };

  match fs::write(path, USB_PORT) {
    Ok(()) => {
      log::info!("USB/LAN {}", if status == Status::Enabled { "enabled" } else { "disabled" });
      status
    },
    Err(e) => {
      log::warn!("Failed to write to {path}: {e}");
      get_usb_status()
    },
  }
}

// ── HDMI ────────────────────────────────────────────────────────────────────
// Turning off HDMI saves ~30mA on the Pi Zero 2W.
// On modern kernels (Bookworm+) the vc4 DRM driver exposes DPMS via
// /sys/class/drm/card?-HDMI-A-1/dpms. Alternatively, `tvservice -o`
// and `tvservice -p` work on older firmware.

/// Read whether the HDMI output is currently on.
pub fn get_hdmi_status() -> Status {
  if let Some(dpms) = read_hdmi_dpms() {
    if dpms.trim() == "On" { Status::Enabled } else { Status::Disabled }
  } else {
    // Fallback: assume enabled if we can't determine
    Status::Enabled
  }
}

/// Enable or disable the HDMI output. Returns the resulting status.
pub fn set_hdmi_status(status: Status) -> Status {
  // Try KMS/DRM dpms sysfs first
  if let Some(dpms_path) = find_hdmi_dpms_path() {
    let value = match status {
      Status::Enabled => "On",
      Status::Disabled => "Off",
    };
    if fs::write(&dpms_path, value).is_ok() {
      log::info!("HDMI {}", if status == Status::Enabled { "enabled" } else { "disabled" });
      return status;
    }
  }

  // Fallback to tvservice (works on older firmware)
  let args = match status {
    Status::Disabled => vec!["-o"],
    Status::Enabled => vec!["-p"],
  };

  match Command::new("tvservice").args(&args).output() {
    Ok(output) if output.status.success() => {
      log::info!("HDMI {} via tvservice",
                 if status == Status::Enabled { "enabled" } else { "disabled" });
      status
    },
    Ok(output) => {
      let stderr = String::from_utf8_lossy(&output.stderr);
      log::warn!("tvservice failed: {}", stderr.trim());
      get_hdmi_status()
    },
    Err(e) => {
      log::warn!("tvservice not found: {e}");
      get_hdmi_status()
    },
  }
}

/// Find the sysfs DPMS path for the HDMI connector (card?-HDMI-A-1).
fn find_hdmi_dpms_path() -> Option<std::path::PathBuf> {
  let drm_dir = std::path::Path::new("/sys/class/drm");
  if !drm_dir.exists() {
    return None;
  }
  for entry in fs::read_dir(drm_dir).ok()? {
    let entry = entry.ok()?;
    let name = entry.file_name();
    if name.to_string_lossy().contains("HDMI") {
      let dpms = entry.path().join("dpms");
      if dpms.exists() {
        return Some(dpms);
      }
    }
  }
  None
}

/// Read the current HDMI DPMS state from sysfs.
fn read_hdmi_dpms() -> Option<String> {
  let path = find_hdmi_dpms_path()?;
  fs::read_to_string(path).ok()
}

// ── Bluetooth ───────────────────────────────────────────────────────────────
// Disabling Bluetooth saves ~20mA. The recommended way is to add
// `dtoverlay=disable-bt` to /boot/firmware/config.txt and reboot.
// At runtime we can use `rfkill` to soft-block it without a reboot.

const RFKILL_BLUETOOTH: &str = "bluetooth";

/// Read whether Bluetooth is currently enabled (not soft-blocked by rfkill).
pub fn get_bluetooth_status() -> Status {
  let output = match Command::new("rfkill").args(["list", RFKILL_BLUETOOTH]).output() {
    Ok(o) => o,
    Err(e) => {
      log::debug!("rfkill not available: {e}");
      return Status::Enabled; // assume enabled if we can't check
    },
  };

  let stdout = String::from_utf8_lossy(&output.stdout);
  if stdout.contains("Soft blocked: yes") {
    Status::Disabled
  } else {
    Status::Enabled
  }
}

/// Enable or disable Bluetooth via rfkill. Returns the resulting status.
/// For a persistent disable across reboots, add `dtoverlay=disable-bt` to
/// /boot/firmware/config.txt.
pub fn set_bluetooth_status(status: Status) -> Status {
  let action = match status {
    Status::Enabled => "unblock",
    Status::Disabled => "block",
  };

  match Command::new("rfkill").args([action, RFKILL_BLUETOOTH]).output() {
    Ok(output) if output.status.success() => {
      log::info!("Bluetooth {}", if status == Status::Enabled { "enabled" } else { "disabled" });
      status
    },
    Ok(output) => {
      let stderr = String::from_utf8_lossy(&output.stderr);
      log::warn!("rfkill {action} bluetooth failed: {}", stderr.trim());
      get_bluetooth_status()
    },
    Err(e) => {
      log::warn!("rfkill not found: {e}");
      get_bluetooth_status()
    },
  }
}
