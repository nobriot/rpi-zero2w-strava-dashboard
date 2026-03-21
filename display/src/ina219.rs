use crate::errors::DisplayError;
use rppal::i2c::I2c;

/// INA219 default I2C address on the RPi Zero PhotoPainter UPS board.
const INA219_ADDR: u16 = 0x43;

// INA219 register addresses
const REG_CONFIG: u8 = 0x00;
const REG_BUS_VOLTAGE: u8 = 0x02;
const REG_CURRENT: u8 = 0x04;
const REG_CALIBRATION: u8 = 0x05;

// Calibration for 16V/5A range with 0.01Ω shunt resistor
// (matches Waveshare UPS board Python reference).
const CAL_VALUE: u16 = 26868;
const CURRENT_LSB: f32 = 0.1524; // mA per bit

/// Battery status from the INA219 UPS monitor.
#[derive(Debug, Clone)]
pub struct BatteryStatus {
  pub voltage:     f32,
  pub current_ma:  f32,
  pub percentage:  u8,
  pub is_charging: bool,
}

pub struct Ina219 {
  i2c: I2c,
}

impl Ina219 {
  pub fn new() -> Result<Self, DisplayError> {
    let mut i2c = I2c::new().map_err(|e| DisplayError::I2c(e.to_string()))?;
    i2c.set_slave_address(INA219_ADDR).map_err(|e| DisplayError::I2c(e.to_string()))?;
    let mut ina = Self { i2c };
    ina.calibrate()?;
    Ok(ina)
  }

  /// Write calibration and config registers (16V range, gain /2, 12-bit
  /// 32-sample averaging, continuous shunt+bus measurement). Matches the
  /// Python `set_calibration_16V_5A()`.
  fn calibrate(&mut self) -> Result<(), DisplayError> {
    self.write_register(REG_CALIBRATION, CAL_VALUE)?;

    // BRNG=16V(0) | Gain=/2(01) | BADC=12bit-32s(1101) | SADC=12bit-32s(1101) |
    // Mode=shunt+bus continuous(111) = 0x0DEF
    let config: u16 = 0x0DEF;
    self.write_register(REG_CONFIG, config)?;
    Ok(())
  }

  pub fn read_status(&mut self) -> Result<BatteryStatus, DisplayError> {
    // Re-write calibration before reading (the Python reference does this too)
    self.write_register(REG_CALIBRATION, CAL_VALUE)?;

    let voltage = self.read_bus_voltage()?;
    let current = self.read_current()?;
    // Positive current = charging, negative = discharging (load)
    let is_charging = current > 0.0;
    let percentage = voltage_to_percentage(voltage);

    Ok(BatteryStatus { voltage,
                       current_ma: current,
                       percentage,
                       is_charging })
  }

  fn read_bus_voltage(&mut self) -> Result<f32, DisplayError> {
    let raw = self.read_register(REG_BUS_VOLTAGE)?;
    // Bus voltage register: bits [15:3] contain the voltage, LSB = 4mV
    let voltage = ((raw >> 3) as f32) * 0.004;
    Ok(voltage)
  }

  fn read_current(&mut self) -> Result<f32, DisplayError> {
    let raw = self.read_register(REG_CURRENT)?;
    // Current register is signed; convert with calibrated LSB
    let current = (raw as i16) as f32 * CURRENT_LSB;
    Ok(current)
  }

  fn read_register(&mut self, reg: u8) -> Result<u16, DisplayError> {
    let mut buf = [0u8; 2];
    self.i2c.write(&[reg]).map_err(|e| DisplayError::I2c(e.to_string()))?;
    self.i2c.read(&mut buf).map_err(|e| DisplayError::I2c(e.to_string()))?;
    Ok(u16::from_be_bytes(buf))
  }

  fn write_register(&mut self, reg: u8, value: u16) -> Result<(), DisplayError> {
    let bytes = value.to_be_bytes();
    self.i2c.write(&[reg, bytes[0], bytes[1]]).map_err(|e| DisplayError::I2c(e.to_string()))?;
    Ok(())
  }
}

/// Map battery voltage to percentage using a linear scale.
/// 3.0V = 0%, 4.2V = 100% (matches Waveshare UPS Python reference).
fn voltage_to_percentage(voltage: f32) -> u8 {
  let pct = (voltage - 3.0) / 1.2 * 100.0;
  pct.clamp(0.0, 100.0) as u8
}
