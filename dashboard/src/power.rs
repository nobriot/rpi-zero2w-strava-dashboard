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

#[allow(dead_code)]
mod refactor {
  use std::fs;
  use std::process::Command;

  /// PowerMode for the application / RPi
  #[derive(PartialEq)]
  pub enum Mode {
    /// The RPi runs normally, using all the power it wants Typically good when
    /// connected to the power supply
    Normal,
    /// The RPi is running on battery, and should try to save as much power as
    /// possible. Note that WiFi will get disabled in this mode
    LowPower,
    /// The RPi will halt, and can only be booted again with a power cycle or
    /// e.g. shorting the RUN pad
    Halted,
  }

  pub struct PowerManager {
    current_mode:  Mode,
    wifi_disabled: Option<bool>,
  }

  impl PowerManager {
    pub fn new(mode: Mode) -> Self {
      let instance = Self { current_mode:  mode,
                            wifi_disabled: None, };
      match instance.current_mode {
        Mode::Normal => {},
        Mode::LowPower => todo!(),
        Mode::Halted => todo!(),
      }
      instance
    }

    /// Disable non-essential peripherals to save power during sleep.
    /// WiFi is kept alive so the user can SSH in during the linger window;
    /// call [`disable_wifi`] after the linger period.
    pub fn set_mode(&mut self, mode: Mode) {
      if self.current_mode == mode {
        return;
      }
      self.current_mode = mode;
      todo!();
    }

    /// Set the DS3231 wake alarm and halt the Pi.
    /// The DS3231 INT pin (wired to the RUN pad) will reboot
    /// the Pi when the alarm fires.
    fn set_rtc_alarm_and_halt(self, sleep_secs: u64) -> Result<(), ()> {
      // Clear any pending alarm
      let _ = fs::write("/sys/class/rtc/rtc0/wakealarm", "0");

      // Set alarm
      let alarm = format!("+{sleep_secs}");
      match fs::write("/sys/class/rtc/rtc0/wakealarm", &alarm) {
        Ok(_) => log::info!("RTC alarm set for {sleep_secs}s from now"),
        Err(e) => {
          // Don't shut down if we can't set the alarm
          log::error!("Failed to set RTC alarm: {e}");
          return Err(());
        },
      }

      log::info!("Halting — DS3231 INT -> RUN pad will reboot on alarm");

      // Sync filesystem before halting
      let sync_result = Command::new("sync").status();
      match sync_result {
        Ok(exit_code) => {
          if !exit_code.success() {
            log::error!("Failed to sync before shutdown: {exit_code}");
          }
        },
        Err(e) => log::error!("Cannot get exit status for sync before shutdown: {e}"),
      }

      // Halt
      let shutdown_result = Command::new("sudo").args(["shutdown", "-h", "now"]).status();
      match shutdown_result {
        Ok(exit_code) => {
          if !exit_code.success() {
            log::error!("Failed to sync before shutdown: {exit_code}");
            return Err(());
          }
        },
        Err(e) => {
          log::error!("Cannot get exit status for sync before shutdown: {e}");
          return Err(());
        },
      }

      Ok(())
    }
  }
}

/// Manages non-essential peripherals (Bluetooth, USB/LAN, WiFi), tracking
/// state to avoid redundant operations on cycles.
pub struct Peripherals {
  low_power:     Option<bool>,
  wifi_disabled: bool,
}

impl Peripherals {
  pub fn new() -> Self {
    Self { low_power:     None,
           wifi_disabled: false, }
  }

  /// Disable non-essential peripherals to save power during sleep.
  /// WiFi is kept alive so the user can SSH in during the linger window;
  /// call [`disable_wifi`] after the linger period.
  pub fn set_low_power(&mut self) {
    if let Some(lp) = self.low_power
       && lp
    {
      return;
    }
    log::info!("Disabling non-essential peripherals for low-power usage");
    set_bluetooth_status(Status::Disabled);
    set_usb_status(Status::Disabled);
    self.low_power = Some(true);
  }

  /// Disable WiFi via rfkill. Called after the linger window when on battery
  /// to save power during the long sleep.
  pub fn disable_wifi(&mut self) {
    if self.wifi_disabled {
      return;
    }
    set_wifi_status(Status::Disabled);
    self.wifi_disabled = true;
  }

  /// Re-enable WiFi via rfkill. Called before the next fetch cycle so the
  /// network is available for Strava API calls.
  pub fn enable_wifi(&mut self) {
    if !self.wifi_disabled {
      return;
    }
    set_wifi_status(Status::Enabled);
    self.wifi_disabled = false;
  }

  /// Re-enable peripherals before the next active cycle.
  pub fn set_normal(&mut self) {
    self.enable_wifi();
    if let Some(lp) = self.low_power
       && !lp
    {
      return;
    }
    log::info!("Re-enabling peripherals");
    set_bluetooth_status(Status::Enabled);
    set_usb_status(Status::Enabled);
    self.low_power = Some(false);
  }
}

impl Drop for Peripherals {
  fn drop(&mut self) {
    if self.wifi_disabled {
      set_wifi_status(Status::Enabled);
    }
  }
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

/// Discover USB device names from /sys/bus/usb/devices/.
/// Returns entries like "1-0:1.0", "usb1", etc.
fn usb_device_names() -> Vec<String> {
  let devices_dir = std::path::Path::new("/sys/bus/usb/devices");
  let Ok(entries) = fs::read_dir(devices_dir) else {
    return Vec::new();
  };
  entries.filter_map(|e| {
           let entry = e.ok()?;
           let name = entry.file_name().into_string().ok()?;
           // Only include entries that have a uevent file
           if entry.path().join("uevent").exists() { Some(name) } else { None }
         })
         .collect()
}

/// Enable or disable all USB devices by writing to the bind/unbind sysfs files.
fn set_usb_status(status: Status) {
  let path = match status {
    Status::Enabled => USB_BIND,
    Status::Disabled => USB_UNBIND,
  };

  let names = usb_device_names();
  if names.is_empty() {
    log::debug!("No USB devices found in /sys/bus/usb/devices/");
    return;
  }

  for name in &names {
    match fs::write(path, name) {
      Ok(()) => {
        log::debug!("USB device {name} {}",
                    if status == Status::Enabled { "bound" } else { "unbound" });
      },
      Err(e) => {
        // ENODEV / EBUSY are expected if already in the target state
        log::debug!("Failed to write {name} to {path}: {e}");
      },
    }
  }

  log::info!("USB {}", if status == Status::Enabled { "enabled" } else { "disabled" });
}

// ── Bluetooth ───────────────────────────────────────────────────────────────
// Disabling Bluetooth saves ~20mA. The recommended way is to add
// `dtoverlay=disable-bt` to /boot/firmware/config.txt and reboot.
// At runtime we can use `rfkill` to soft-block it without a reboot.

const RFKILL_BLUETOOTH: &str = "bluetooth";
const RFKILL_WIFI: &str = "wifi";

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
fn set_bluetooth_status(status: Status) -> Status {
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

// -- WiFi --------------------------------------------------------------------
// Soft-blocking WiFi via rfkill saves significant power when the radio is not
// needed (between fetch cycles on battery).

fn set_wifi_status(status: Status) {
  let action = match status {
    Status::Enabled => "unblock",
    Status::Disabled => "block",
  };

  match Command::new("rfkill").args([action, RFKILL_WIFI]).output() {
    Ok(output) if output.status.success() => {
      log::info!("WiFi {}", if status == Status::Enabled { "enabled" } else { "disabled" });
    },
    Ok(output) => {
      let stderr = String::from_utf8_lossy(&output.stderr);
      log::warn!("rfkill {action} wifi failed: {}", stderr.trim());
    },
    Err(e) => {
      log::warn!("rfkill not found: {e}");
    },
  }
}
