use crate::errors::DisplayError;
use rppal::i2c::I2c;

/// INA219 default I2C address on the RPi Zero PhotoPainter UPS board.
const INA219_ADDR: u16 = 0x43;

// INA219 register addresses
const REG_BUS_VOLTAGE: u8 = 0x02;
const REG_CURRENT: u8 = 0x04;

/// Battery status from the INA219 UPS monitor.
#[derive(Debug, Clone)]
pub struct BatteryStatus {
    pub voltage: f32,
    pub current_ma: f32,
    pub percentage: u8,
    pub is_charging: bool,
}

pub struct Ina219 {
    i2c: I2c,
}

impl Ina219 {
    pub fn new() -> Result<Self, DisplayError> {
        let mut i2c = I2c::new().map_err(|e| DisplayError::I2c(e.to_string()))?;
        i2c.set_slave_address(INA219_ADDR)
            .map_err(|e| DisplayError::I2c(e.to_string()))?;
        Ok(Self { i2c })
    }

    pub fn read_status(&mut self) -> Result<BatteryStatus, DisplayError> {
        let voltage = self.read_bus_voltage()?;
        let current = self.read_current()?;
        let is_charging = current > 0.0;
        let percentage = voltage_to_percentage(voltage, is_charging);

        Ok(BatteryStatus {
            voltage,
            current_ma: current,
            percentage,
            is_charging,
        })
    }

    fn read_bus_voltage(&mut self) -> Result<f32, DisplayError> {
        let raw = self.read_register(REG_BUS_VOLTAGE)?;
        // Bus voltage register: bits [15:3] contain the voltage, LSB = 4mV
        let voltage = ((raw >> 3) as f32) * 0.004;
        Ok(voltage)
    }

    fn read_current(&mut self) -> Result<f32, DisplayError> {
        let raw = self.read_register(REG_CURRENT)?;
        // Current register is signed, LSB depends on calibration.
        // Default calibration: ~1mA per bit.
        let current = (raw as i16) as f32;
        Ok(current)
    }

    fn read_register(&mut self, reg: u8) -> Result<u16, DisplayError> {
        let mut buf = [0u8; 2];
        self.i2c
            .write(&[reg])
            .map_err(|e| DisplayError::I2c(e.to_string()))?;
        self.i2c
            .read(&mut buf)
            .map_err(|e| DisplayError::I2c(e.to_string()))?;
        Ok(u16::from_be_bytes(buf))
    }
}

/// Map battery voltage to percentage.
/// Lookup table derived from the Ibis Dash / Waveshare UPS demo.
fn voltage_to_percentage(voltage: f32, is_charging: bool) -> u8 {
    if is_charging {
        match voltage {
            v if v >= 4.10 => 100,
            v if v >= 4.00 => 90,
            v if v >= 3.90 => 75,
            v if v >= 3.80 => 55,
            v if v >= 3.70 => 40,
            v if v >= 3.60 => 25,
            v if v >= 3.50 => 15,
            _ => 5,
        }
    } else {
        match voltage {
            v if v >= 4.05 => 100,
            v if v >= 3.95 => 90,
            v if v >= 3.85 => 75,
            v if v >= 3.75 => 55,
            v if v >= 3.65 => 35,
            v if v >= 3.55 => 20,
            v if v >= 3.45 => 10,
            _ => 5,
        }
    }
}
