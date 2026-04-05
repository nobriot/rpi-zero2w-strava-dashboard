use rppal::gpio::{Gpio, Level};
use rppal::i2c::I2c;
use std::fs;

const GPIO_INT_PIN: u8 = 4;
const I2C_ADDR: u16 = 0x68;
const STATUS_REG: u8 = 0x0F;
const WAKEALARM: &str = "/sys/class/rtc/rtc0/wakealarm";

/// Clear a pending DS3231 alarm interrupt if GPIO 4 (INT/SQW) is asserted low.
///
/// After an rtcwake cycle, the alarm flag stays set and INT/SQW stays low
/// until explicitly cleared. This must happen early at startup so the pin
/// is free for the next wake cycle.
pub fn clear_alarm_if_pending() {
  let pin_low = match Gpio::new().and_then(|gpio| gpio.get(GPIO_INT_PIN)) {
    Ok(pin) => pin.into_input_pullup().read() == Level::Low,
    Err(e) => {
      log::debug!("ds3231: cannot read GPIO {GPIO_INT_PIN}: {e}");
      return;
    },
  };

  if !pin_low {
    log::debug!("ds3231: GPIO {GPIO_INT_PIN} is high, no pending alarm");
    return;
  }

  log::info!("ds3231: GPIO {GPIO_INT_PIN} is low -- clearing pending alarm");

  // Clear alarm flags in status register (preserve EN32kHz bit 3)
  let result = (|| -> Result<(), rppal::i2c::Error> {
    let mut i2c = I2c::new()?;
    i2c.set_slave_address(I2C_ADDR)?;
    let status = i2c.smbus_read_byte(STATUS_REG)?;
    let cleared = status & !0x03; // clear A1F and A2F
    i2c.smbus_write_byte(STATUS_REG, cleared)?;
    Ok(())
  })();

  if let Err(e) = result {
    log::warn!("ds3231: I2C error clearing alarm: {e}");
  }

  // Also clear the kernel wakealarm
  if let Err(e) = fs::write(WAKEALARM, "0") {
    log::debug!("ds3231: failed to clear wakealarm: {e}");
  }

  log::info!("ds3231: alarm cleared");
}
