use crate::args::Args;
use crate::config::{Config, PowerConfig};
use crate::cycle;
use crate::errors::Result;
use crate::heartbeat::write_heartbeat;
use crate::power::{self, PowerManager, RestOutcome};
use crate::schedule::{self, SleepPlan};

/// Main event loop: fetch -> render -> sleep, repeat.
pub fn run(mut config: Config, args: Args) -> Result<()> {
  if args.kiosk {
    return run_kiosk(config, args);
  }

  let mut power_mgr = PowerManager::new(config.power.tpl5110_done_pin);
  let mut client: Option<strava::client::Client> = None;

  loop {
    power_mgr.enable_wifi();

    let battery = power::read_battery();
    if schedule::should_skip_cycle(&config.power, battery.as_ref()) {
      log::info!("Quiet hours with TPL5110 -- skipping cycle");
    } else {
      cycle::run(&mut config, &mut client, &args, &mut power_mgr)?;
      write_heartbeat("Regular start");
    }

    if args.once {
      return Ok(());
    }

    if rest_between_cycles(&config.power, &mut power_mgr) {
      return Ok(());
    }
  }
}

/// Sleep between dashboard cycles. On power, polls so a cable unplug is
/// noticed within `power_poll_interval_secs`; on battery, lingers and then
/// either sleeps or shuts down.
///
/// Returns `true` when a hard shutdown was initiated and the main loop
/// should exit.
fn rest_between_cycles(power: &PowerConfig, mgr: &mut PowerManager) -> bool {
  let battery = power::read_battery();
  match schedule::plan(power, battery.as_ref()) {
    SleepPlan::OnPower { sleep_secs } => {
      match mgr.rest_on_power(sleep_secs, power.power_poll_interval_secs) {
        RestOutcome::Refresh => false,
        RestOutcome::PowerLost => rest_on_battery_now(power, mgr),
      }
    },
    SleepPlan::Battery { sleep_secs, linger_secs } => {
      let battery_pct = battery.as_ref().map(|b| b.percentage());
      mgr.rest_on_battery(power, sleep_secs, linger_secs, battery_pct)
    },
  }
}

/// Re-evaluate the schedule with a fresh battery read and run the battery
/// rest path. If power happens to be back, just refresh on the next cycle.
fn rest_on_battery_now(power: &PowerConfig, mgr: &mut PowerManager) -> bool {
  let battery = power::read_battery();
  match schedule::plan(power, battery.as_ref()) {
    SleepPlan::Battery { sleep_secs, linger_secs } => {
      let battery_pct = battery.as_ref().map(|b| b.percentage());
      mgr.rest_on_battery(power, sleep_secs, linger_secs, battery_pct)
    },
    SleepPlan::OnPower { .. } => false,
  }
}

/// Kiosk mode: no power management, no quiet hours, no e-paper.
/// Refreshes the PNG on the charging interval, stays awake with WiFi on.
fn run_kiosk(mut config: Config, args: Args) -> Result<()> {
  let mut power_mgr = PowerManager::new(None);
  let mut client: Option<strava::client::Client> = None;

  loop {
    cycle::run(&mut config, &mut client, &args, &mut power_mgr)?;
    write_heartbeat("Kiosk start");

    if args.once {
      return Ok(());
    }

    let sleep_secs = config.power.charging_interval_secs;
    log::info!("Kiosk mode -- sleeping {sleep_secs}s before next refresh");
    std::thread::sleep(std::time::Duration::from_secs(sleep_secs));
  }
}
