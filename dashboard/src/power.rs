use std::fs;
use std::process::Command;

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
/// machine). This is non-fatal -- the caller uses `None` to mean "on
/// external power / no battery".
pub fn read_battery() -> Option<common::BatteryStatus> {
  match crate::ina219::Ina219::new().and_then(|mut ina| ina.read_status()) {
    Ok(status) => {
      log::info!("Battery: {status}");
      Some(status)
    },
    Err(e) => {
      log::debug!("Battery monitor unavailable: {e}");
      None
    },
  }
}

/// Power mode for the RPi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
  /// All peripherals enabled. Used when on external power or during active
  /// fetch cycles.
  Normal,
  /// Battery-saving mode. Bluetooth and USB are disabled; WiFi stays on so
  /// the user can SSH in during the linger window.
  LowPower,
}

/// Manages RPi peripheral power state (WiFi, Bluetooth, USB) and provides
/// rtcwake shutdown.
///
/// WiFi can be independently toggled within any mode -- `disable_wifi()` is
/// typically called after the linger window when the radio is no longer
/// needed.
///
/// On drop, WiFi is re-enabled so the Pi stays reachable even after a crash
/// or early return.
pub struct PowerManager {
  mode:         Mode,
  wifi_blocked: bool,
  bt_blocked:   bool,
  usb_unbound:  bool,
  tpl5110_done: Option<rppal::gpio::OutputPin>,
}

impl PowerManager {
  pub fn new(tpl5110_done_pin: Option<u8>) -> Self {
    let tpl5110_done = tpl5110_done_pin.and_then(|pin| {
                         match rppal::gpio::Gpio::new().and_then(|gpio| gpio.get(pin)) {
                           Ok(p) => {
                             let out = p.into_output_low();
                             log::info!("TPL5110: DONE pin GPIO {pin} initialized LOW");
                             Some(out)
                           },
                           Err(e) => {
                             log::warn!("TPL5110: failed to initialize GPIO {pin}: {e}");
                             None
                           },
                         }
                       });
    Self { mode: Mode::Normal,
           wifi_blocked: false,
           bt_blocked: false,
           usb_unbound: false,
           tpl5110_done }
  }

  /// Current power mode.
  #[allow(dead_code)]
  pub fn mode(&self) -> Mode {
    self.mode
  }

  /// Enable all peripherals. No-op if already in Normal mode with
  /// everything enabled.
  pub fn set_normal(&mut self) {
    self.enable_wifi();
    if self.mode == Mode::Normal && !self.bt_blocked && !self.usb_unbound {
      return;
    }
    log::info!("Power: switching to Normal mode");
    self.set_bluetooth(true);
    self.set_usb(true);
    self.mode = Mode::Normal;
  }

  /// Disable non-essential peripherals (Bluetooth, USB) to save power.
  /// WiFi is kept alive for SSH access during the linger window.
  pub fn set_low_power(&mut self) {
    if self.mode == Mode::LowPower && self.bt_blocked && self.usb_unbound {
      return;
    }
    log::info!("Power: switching to LowPower mode");
    self.set_bluetooth(false);
    self.set_usb(false);
    self.mode = Mode::LowPower;
  }

  /// Disable WiFi via rfkill. Called after the linger window to save power
  /// during the long sleep.
  /// We don't disable wifi if we have a tpl5110 pin configured
  pub fn disable_wifi(&mut self) {
    if self.wifi_blocked {
      return;
    }
    if self.tpl5110_done.is_some() {
      return;
    }
    rfkill("block", "wifi");
    self.wifi_blocked = true;
  }

  /// Re-enable WiFi via rfkill. Called before each fetch cycle so the
  /// network is available for Strava API calls.
  pub fn enable_wifi(&mut self) {
    if !self.wifi_blocked {
      return;
    }
    rfkill("unblock", "wifi");
    self.wifi_blocked = false;
    // Wait a few seconds for the connectivity to be available
    std::thread::sleep(std::time::Duration::from_secs(5));
  }

  /// Shut down the machine.
  ///
  /// If a TPL5110 DONE pin is configured, asserts it high first so the
  /// TPL5110 can cut power. Then falls back to `shutdown -h now`.
  /// Returns `true` if shutdown was initiated.
  pub fn shutdown(&mut self) -> bool {
    log::info!("Syncing filesystems before shutdown");
    let _ = Command::new("sync").status();

    if let Some(ref mut pin) = self.tpl5110_done {
      log::info!("TPL5110: asserting DONE high");
      pin.set_high();
      // TPL5110 should cut power within milliseconds.
      // If still alive after 5s, fall through to software shutdown.
      std::thread::sleep(std::time::Duration::from_secs(5));
      log::warn!("TPL5110: still alive after DONE -- falling back to shutdown");
      pin.set_low();
    }

    match Command::new("sudo").args(["shutdown", "-h", "now"]).status() {
      Ok(status) if status.success() => {
        log::info!("shutdown -h now initiated");
        true
      },
      Ok(status) => {
        log::warn!("shutdown -h now failed (exit {})", status);
        false
      },
      Err(e) => {
        log::warn!("Failed to run shutdown: {e}");
        false
      },
    }
  }

  fn set_bluetooth(&mut self, enable: bool) {
    if enable != self.bt_blocked {
      return;
    }
    let action = if enable { "unblock" } else { "block" };
    rfkill(action, "bluetooth");
    self.bt_blocked = !enable;
  }

  fn set_usb(&mut self, enable: bool) {
    let path = if enable { USB_BIND } else { USB_UNBIND };

    let names = usb_device_names();
    if names.is_empty() {
      log::debug!("No USB devices found in /sys/bus/usb/devices/");
      self.usb_unbound = !enable;
      return;
    }

    for name in &names {
      match fs::write(path, name) {
        Ok(()) => {
          log::debug!("USB device {name} {}", if enable { "bound" } else { "unbound" });
        },
        Err(e) => {
          // ENODEV / EBUSY are expected if already in the target state
          log::debug!("Failed to write {name} to {path}: {e}");
        },
      }
    }

    log::info!("USB {}", if enable { "enabled" } else { "disabled" });
    self.usb_unbound = !enable;
  }
}

impl Drop for PowerManager {
  fn drop(&mut self) {
    if self.wifi_blocked {
      rfkill("unblock", "wifi");
    }
  }
}

// ---------------------------------------------------------------------------
// Low-level peripheral helpers
// ---------------------------------------------------------------------------

const USB_BIND: &str = "/sys/bus/usb/drivers/usb/bind";
const USB_UNBIND: &str = "/sys/bus/usb/drivers/usb/unbind";

/// Discover USB device names from /sys/bus/usb/devices/.
fn usb_device_names() -> Vec<String> {
  let devices_dir = std::path::Path::new("/sys/bus/usb/devices");
  let Ok(entries) = fs::read_dir(devices_dir) else {
    return Vec::new();
  };
  entries.filter_map(|e| {
           let entry = e.ok()?;
           let name = entry.file_name().into_string().ok()?;
           if entry.path().join("uevent").exists() { Some(name) } else { None }
         })
         .collect()
}

/// Run `rfkill <action> <device>` and log the result.
fn rfkill(action: &str, device: &str) {
  match Command::new("rfkill").args([action, device]).output() {
    Ok(output) if output.status.success() => {
      log::info!("{device} {}", if action == "unblock" { "enabled" } else { "disabled" });
    },
    Ok(output) => {
      let stderr = String::from_utf8_lossy(&output.stderr);
      log::warn!("rfkill {action} {device} failed: {}", stderr.trim());
    },
    Err(e) => {
      log::warn!("rfkill not found: {e}");
    },
  }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn new_starts_in_normal_mode() {
    let pm = PowerManager::new(None);
    assert_eq!(pm.mode(), Mode::Normal);
    assert!(!pm.wifi_blocked);
    assert!(!pm.bt_blocked);
    assert!(!pm.usb_unbound);
  }

  #[test]
  fn set_low_power_changes_mode() {
    let mut pm = PowerManager::new(None);
    pm.set_low_power();
    assert_eq!(pm.mode(), Mode::LowPower);
    assert!(pm.bt_blocked);
    assert!(pm.usb_unbound);
    // WiFi stays on in low-power mode
    assert!(!pm.wifi_blocked);
  }

  #[test]
  fn set_normal_restores_mode() {
    let mut pm = PowerManager::new(None);
    pm.set_low_power();
    pm.disable_wifi();
    assert!(pm.wifi_blocked);

    pm.set_normal();
    assert_eq!(pm.mode(), Mode::Normal);
    assert!(!pm.wifi_blocked);
    assert!(!pm.bt_blocked);
    assert!(!pm.usb_unbound);
  }

  #[test]
  fn wifi_toggle_is_independent_of_mode() {
    let mut pm = PowerManager::new(None);
    pm.disable_wifi();
    assert!(pm.wifi_blocked);
    assert_eq!(pm.mode(), Mode::Normal);

    pm.enable_wifi();
    assert!(!pm.wifi_blocked);
  }

  #[test]
  fn redundant_mode_changes_are_noops() {
    let mut pm = PowerManager::new(None);
    pm.set_low_power();
    let bt = pm.bt_blocked;
    let usb = pm.usb_unbound;

    // Second call should not change state
    pm.set_low_power();
    assert_eq!(pm.bt_blocked, bt);
    assert_eq!(pm.usb_unbound, usb);
  }

  #[test]
  fn redundant_wifi_toggles_are_noops() {
    let mut pm = PowerManager::new(None);
    // Already unblocked -- enable_wifi should be a no-op
    pm.enable_wifi();
    assert!(!pm.wifi_blocked);

    pm.disable_wifi();
    // Already blocked -- disable_wifi should be a no-op
    pm.disable_wifi();
    assert!(pm.wifi_blocked);
  }

  #[test]
  fn shutdown_returns_false_on_dev_machine() {
    // On dev machines, shutdown -h now should fail (no sudo/permissions)
    let mut pm = PowerManager::new(None);
    assert!(!pm.shutdown());
  }
}
