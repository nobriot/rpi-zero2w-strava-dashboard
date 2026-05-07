use crate::config::PowerConfig;
use chrono::{Duration, Local, Timelike};
use common::BatteryStatus;

/// What to do after a dashboard cycle completes.
#[derive(Debug)]
pub enum SleepPlan {
  /// On external power (or no battery sensor) -- short sleep, normal mode.
  OnPower { sleep_secs: u64 },
  /// On battery -- sleep or shutdown, with optional linger for SSH.
  Battery { sleep_secs: u64, linger_secs: u64 },
}

/// Compute the sleep plan based on power state and config.
///
/// On power: always OnPower. The device stays awake (`shutdown_after_cycle`
/// and `tpl5110_done_pin` only fire on battery).
pub fn plan(power: &PowerConfig, battery: Option<&BatteryStatus>) -> SleepPlan {
  let on_power = battery.is_none() || battery.is_some_and(|b| b.is_charging());

  if on_power {
    return SleepPlan::OnPower { sleep_secs: power.charging_interval_secs, };
  }

  let sleep_secs = if is_quiet_time(power) {
    let secs = seconds_until_quiet_end(power);
    log::info!("Quiet hours ({:02}:00-{:02}:00) -- sleeping {secs}s until wake",
               power.quiet_hours.start,
               power.quiet_hours.end);
    secs
  } else {
    let secs = seconds_until_next_slot(power, power.sleep_interval_secs);
    log::info!("Battery mode -- sleeping {secs}s (next grid slot)");
    secs
  };

  let linger_secs = power.linger_secs.min(sleep_secs);

  SleepPlan::Battery { sleep_secs, linger_secs }
}

/// Should the dashboard cycle be skipped entirely?
///
/// True when on battery during quiet hours with a hard-shutdown method
/// (TPL5110) available -- no point rendering if nobody will see it.
pub fn should_skip_cycle(power: &PowerConfig, battery: Option<&BatteryStatus>) -> bool {
  let on_power = battery.is_none() || battery.is_some_and(|b| b.is_charging());
  !on_power
  && is_quiet_time(power)
  && (power.tpl5110_done_pin.is_some() || power.shutdown_after_cycle)
}

/// Check whether the current local time falls inside the quiet window.
pub fn is_quiet_time(power: &PowerConfig) -> bool {
  let hour = Local::now().hour();
  let start = power.quiet_hours.start;
  let end = power.quiet_hours.end;

  if start <= end {
    // e.g. quiet 02:00-06:00 (no midnight wrap)
    hour >= start && hour < end
  } else {
    // e.g. quiet 20:00-08:00 (wraps midnight)
    hour >= start || hour < end
  }
}

/// Compute seconds from now until the quiet window ends.
fn seconds_until_quiet_end(power: &PowerConfig) -> u64 {
  let now = Local::now();
  let hour = now.hour();
  let end = power.quiet_hours.end;

  // Hours remaining until the end hour
  let hours_left = if hour < end { end - hour } else { (24 - hour) + end };

  let minutes_left = 60 - now.minute();
  // Subtract one hour because the minutes already cover part of it,
  // but ensure we don't underflow.
  let total_secs = if hours_left > 0 {
    ((hours_left - 1) as u64 * 3600) + (minutes_left as u64 * 60)
  } else {
    minutes_left as u64 * 60
  };

  // At least 60 seconds to avoid a busy-loop from rounding
  total_secs.max(60)
}

/// Compute seconds until the next grid-aligned wake slot.
///
/// The grid is anchored at `quiet_hours.end` each day, with slots
/// spaced `interval_secs` apart.  For example, with quiet end=6 and
/// interval=1200 (20 min), the slots are 06:00, 06:20, 06:40, 07:00, ...
///
/// If the next slot would fall inside quiet hours, returns the seconds
/// until quiet end instead (i.e. the first slot of the next active window).
fn seconds_until_next_slot(power: &PowerConfig, interval_secs: u64) -> u64 {
  let now = Local::now();
  let today_anchor = now.date_naive()
                        .and_hms_opt(power.quiet_hours.end, 0, 0)
                        .expect("valid quiet_hours.end")
                        .and_local_timezone(Local)
                        .single()
                        .expect("unambiguous local time");

  // Use today's anchor if it's in the past, otherwise yesterday's.
  let anchor = if today_anchor <= now {
    today_anchor
  } else {
    today_anchor - Duration::days(1)
  };

  let elapsed = (now - anchor).num_seconds() as u64;
  let remainder = elapsed % interval_secs;
  let next_in = if remainder == 0 { interval_secs } else { interval_secs - remainder };

  // Check whether the target wake time would land in quiet hours.
  let wake_time = now + Duration::seconds(next_in as i64);
  let wake_hour = wake_time.hour();
  let in_quiet = {
    let start = power.quiet_hours.start;
    let end = power.quiet_hours.end;
    if start <= end {
      wake_hour >= start && wake_hour < end
    } else {
      wake_hour >= start || wake_hour < end
    }
  };

  if in_quiet { seconds_until_quiet_end(power) } else { next_in.max(60) }
}

// ---------------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
  use super::*;

  fn power_config() -> PowerConfig {
    PowerConfig { sleep_interval_secs: 10800,
                  charging_interval_secs: 600,
                  linger_secs: 120,
                  ssh_inhibit_below_percent: 60,
                  tpl5110_done_pin: None,
                  ..PowerConfig::default() }
  }

  fn battery(charging: bool) -> BatteryStatus {
    BatteryStatus { voltage:    if charging { 4.1 } else { 3.8 },
                    current_ma: if charging { 300.0 } else { -200.0 },
                    power:      1.0, }
  }

  #[test]
  fn plan_on_power_when_charging() {
    let cfg = power_config();
    let bat = battery(true);
    let p = plan(&cfg, Some(&bat));
    assert!(matches!(p, SleepPlan::OnPower { sleep_secs: 600 }));
  }

  #[test]
  fn plan_on_power_when_no_battery() {
    let cfg = power_config();
    let p = plan(&cfg, None);
    assert!(matches!(p, SleepPlan::OnPower { sleep_secs: 600 }));
  }

  #[test]
  fn plan_on_power_overrides_shutdown_settings() {
    // Even if shutdown_after_cycle / TPL5110 are set, on power the device
    // should stay awake.
    let mut cfg = power_config();
    cfg.shutdown_after_cycle = true;
    cfg.tpl5110_done_pin = Some(16);
    let bat = battery(true);
    assert!(matches!(plan(&cfg, Some(&bat)), SleepPlan::OnPower { .. }));
    assert!(matches!(plan(&cfg, None), SleepPlan::OnPower { .. }));
  }

  #[test]
  fn plan_battery_returns_battery_variant() {
    let cfg = power_config();
    let bat = battery(false);
    let p = plan(&cfg, Some(&bat));
    assert!(matches!(p, SleepPlan::Battery { .. }));
  }

  #[test]
  fn plan_linger_capped_to_sleep() {
    let mut cfg = power_config();
    cfg.linger_secs = 999999; // way more than any sleep duration
    let bat = battery(false);
    let p = plan(&cfg, Some(&bat));
    if let SleepPlan::Battery { sleep_secs, linger_secs, .. } = p {
      assert!(linger_secs <= sleep_secs);
    } else {
      panic!("expected Battery variant");
    }
  }

  #[test]
  fn skip_cycle_requires_tpl5110_and_battery_and_quiet() {
    let mut cfg = power_config();
    let bat = battery(false);

    // No TPL5110 -> never skip
    assert!(!should_skip_cycle(&cfg, Some(&bat)));

    // With TPL5110 but charging -> no skip
    cfg.tpl5110_done_pin = Some(16);
    let charging = battery(true);
    assert!(!should_skip_cycle(&cfg, Some(&charging)));

    // With TPL5110, on battery, but depends on quiet hours (time-dependent)
    // Just verify the function runs without panic
    let _ = should_skip_cycle(&cfg, Some(&bat));
  }

  #[test]
  fn skip_cycle_no_battery_sensor() {
    let mut cfg = power_config();
    cfg.tpl5110_done_pin = Some(16);
    // No battery sensor -> treated as on-power -> no skip
    assert!(!should_skip_cycle(&cfg, None));
  }
}
