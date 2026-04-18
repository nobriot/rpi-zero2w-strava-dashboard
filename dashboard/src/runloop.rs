use crate::args::Args;
use crate::config::Config;
use crate::cycle;
use crate::errors::Result;
use crate::heartbeat::write_heartbeat;
use crate::power::{self, PowerManager};
use crate::schedule::{self, SleepPlan};

/// Main event loop: fetch -> render -> sleep, repeat.
pub fn run(mut config: Config, args: Args) -> Result<()> {
  if args.kiosk {
    return run_kiosk(config, args);
  }

  let mut power_mgr = PowerManager::new(config.power.tpl5110_done_pin);

  loop {
    power_mgr.enable_wifi();

    let battery = power::read_battery();
    if schedule::should_skip_cycle(&config.power, battery.as_ref()) {
      log::info!("Quiet hours with TPL5110 -- skipping cycle");
    } else {
      cycle::run(&mut config, &args, &mut power_mgr)?;
      write_heartbeat("Regular start");
    }

    if args.once {
      return Ok(());
    }

    let battery = power::read_battery();
    match schedule::plan(&config.power, battery.as_ref()) {
      SleepPlan::OnPower { sleep_secs } => power_mgr.rest_on_power(sleep_secs),
      SleepPlan::Battery { sleep_secs, linger_secs } => {
        let battery_pct = battery.as_ref().map(|b| b.percentage());
        if power_mgr.rest_on_battery(&config.power, sleep_secs, linger_secs, battery_pct) {
          return Ok(());
        }
      },
    }
  }
}

/// Kiosk mode: no power management, no quiet hours, no e-paper.
/// Refreshes the PNG on the charging interval, stays awake with WiFi on.
fn run_kiosk(mut config: Config, args: Args) -> Result<()> {
  let mut power_mgr = PowerManager::new(None);

  loop {
    cycle::run(&mut config, &args, &mut power_mgr)?;
    write_heartbeat("Kiosk start");

    if args.once {
      return Ok(());
    }

    let sleep_secs = config.power.charging_interval_secs;
    log::info!("Kiosk mode -- sleeping {sleep_secs}s before next refresh");
    std::thread::sleep(std::time::Duration::from_secs(sleep_secs));
  }
}
