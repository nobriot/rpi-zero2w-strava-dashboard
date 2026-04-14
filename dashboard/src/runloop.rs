use crate::args::Args;
use crate::config::Config;
use crate::cycle;
use crate::errors::Result;
use crate::power::{self, PowerManager};
use crate::schedule::{self, SleepPlan};

/// Main event loop: fetch -> render -> sleep, repeat.
pub fn run(mut config: Config, args: Args) -> Result<()> {
  let mut power_mgr = PowerManager::new(config.power.tpl5110_done_pin);

  loop {
    power_mgr.enable_wifi();

    let battery = power::read_battery();
    if schedule::should_skip_cycle(&config.power, battery.as_ref()) {
      log::info!("Quiet hours with TPL5110 -- skipping cycle");
    } else {
      cycle::run(&mut config, &args, &mut power_mgr)?;
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
