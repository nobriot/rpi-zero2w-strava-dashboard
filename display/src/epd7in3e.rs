use crate::errors::DisplayError;
use log;
use rppal::gpio::{Gpio, InputPin, Level, OutputPin};
use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use std::thread;
use std::time::Duration;

/// Pin assignments for the RPi Zero PhotoPainter board.
/// These differ from the standard Waveshare e-Paper HAT!
const DC_PIN: u8 = 25;
const RST_PIN: u8 = 17;
const BUSY_PIN: u8 = 24;
const PWR_PIN: u8 = 27;

const WIDTH: u16 = 800;
const HEIGHT: u16 = 480;

/// Maximum time to wait for the BUSY pin to go HIGH (ready).
const BUSY_TIMEOUT: Duration = Duration::from_secs(30);

pub struct Epd7in3e {
  spi:  Spi,
  dc:   OutputPin,
  rst:  OutputPin,
  busy: InputPin,
  pwr:  OutputPin,
}

impl Epd7in3e {
  pub const HEIGHT: u16 = HEIGHT;
  pub const WIDTH: u16 = WIDTH;

  /// Initialize the display hardware. Requires root/SPI access.
  pub fn new() -> Result<Self, DisplayError> {
    let gpio = Gpio::new().map_err(|e| DisplayError::Gpio(e.to_string()))?;

    let dc = gpio.get(DC_PIN).map_err(|e| DisplayError::Gpio(e.to_string()))?.into_output();
    let rst = gpio.get(RST_PIN).map_err(|e| DisplayError::Gpio(e.to_string()))?.into_output();
    let busy = gpio.get(BUSY_PIN).map_err(|e| DisplayError::Gpio(e.to_string()))?.into_input();
    let pwr = gpio.get(PWR_PIN).map_err(|e| DisplayError::Gpio(e.to_string()))?.into_output();

    // SPI0, CE0, Mode 0, 10 MHz
    let spi = Spi::new(Bus::Spi0, SlaveSelect::Ss0, 10_000_000, Mode::Mode0)
      .map_err(|e| DisplayError::Spi(e.to_string()))?;

    let mut epd = Self { spi, dc, rst, busy, pwr };

    epd.power_on();
    epd.hardware_reset();
    epd.init_display()?;

    Ok(epd)
  }

  fn power_on(&mut self) {
    self.pwr.set_high();
    thread::sleep(Duration::from_millis(100));
    log::info!("EPD power on");
  }

  fn power_off(&mut self) {
    self.pwr.set_low();
    log::info!("EPD power off");
  }

  fn hardware_reset(&mut self) {
    self.rst.set_high();
    thread::sleep(Duration::from_millis(20));
    self.rst.set_low();
    thread::sleep(Duration::from_millis(2));
    self.rst.set_high();
    thread::sleep(Duration::from_millis(20));
    log::debug!("EPD hardware reset complete");
  }

  fn wait_busy(&self) -> Result<(), DisplayError> {
    log::debug!("Waiting for EPD busy...");
    let start = std::time::Instant::now();
    // BUSY is LOW when the display is busy, HIGH when ready
    while self.busy.read() == Level::Low {
      if start.elapsed() > BUSY_TIMEOUT {
        return Err(DisplayError::BusyTimeout);
      }
      thread::sleep(Duration::from_millis(20));
    }
    log::debug!("EPD ready (waited {:?})", start.elapsed());
    Ok(())
  }

  fn send_command(&mut self, cmd: u8) -> Result<(), DisplayError> {
    self.dc.set_low();
    self.spi.write(&[cmd]).map_err(|e| DisplayError::Spi(e.to_string()))?;
    Ok(())
  }

  fn send_data(&mut self, data: &[u8]) -> Result<(), DisplayError> {
    self.dc.set_high();
    // Send in chunks to avoid SPI buffer limits
    for chunk in data.chunks(4096) {
      self.spi.write(chunk).map_err(|e| DisplayError::Spi(e.to_string()))?;
    }
    Ok(())
  }

  /// Initialize the epd7in3e display controller.
  /// Command sequence ported from Waveshare C driver.
  fn init_display(&mut self) -> Result<(), DisplayError> {
    self.wait_busy()?;

    // Software reset
    self.send_command(0x12)?;
    thread::sleep(Duration::from_millis(100));
    self.wait_busy()?;

    // Set gate driver output
    self.send_command(0x01)?;
    self.send_data(&[0xDF, 0x01, 0x00])?;

    // Set gate driving voltage
    self.send_command(0x03)?;
    self.send_data(&[0x00])?;

    // Set source driving voltage
    self.send_command(0x04)?;
    self.send_data(&[0x41, 0xA8, 0x32])?;

    // Data entry mode: Y decrement, X increment
    self.send_command(0x11)?;
    self.send_data(&[0x03])?;

    // Set RAM X address range
    self.send_command(0x44)?;
    self.send_data(&[0x00, 0x31])?; // 0x00 to 0x31 (50 bytes = 800/16)

    // Set RAM Y address range
    self.send_command(0x45)?;
    self.send_data(&[0xDF, 0x01, 0x00, 0x00])?; // 0x01DF (479) to 0x0000

    // Border waveform control
    self.send_command(0x3C)?;
    self.send_data(&[0x01])?;

    // Temperature sensor
    self.send_command(0x18)?;
    self.send_data(&[0x80])?;

    // Display update control
    self.send_command(0x22)?;
    self.send_data(&[0xCF])?;

    log::info!("EPD initialized ({}x{})", WIDTH, HEIGHT);
    Ok(())
  }

  /// Display a 6-color packed image buffer.
  /// Buffer format: 2 pixels per byte (high nibble = left pixel, low nibble =
  /// right pixel). Colors: 0=Black, 1=White, 2=Green, 3=Blue, 4=Red,
  /// 5=Yellow. Expected length: WIDTH * HEIGHT / 2 = 192000 bytes.
  pub fn display_image(&mut self, buf: &[u8]) -> Result<(), DisplayError> {
    let expected_len = (WIDTH as usize * HEIGHT as usize) / 2;
    if buf.len() != expected_len {
      return Err(DisplayError::Render(format!("Image buffer size mismatch: expected {expected_len}, got {}",
                                              buf.len())));
    }

    log::info!("Sending image data to EPD...");

    // Set RAM X counter
    self.send_command(0x4E)?;
    self.send_data(&[0x00])?;

    // Set RAM Y counter
    self.send_command(0x4F)?;
    self.send_data(&[0xDF, 0x01])?;

    // Write RAM
    self.send_command(0x24)?;
    self.send_data(buf)?;

    // Trigger display refresh
    self.send_command(0x22)?;
    self.send_data(&[0xCF])?;
    self.send_command(0x20)?;
    self.wait_busy()?;

    log::info!("EPD refresh complete");
    Ok(())
  }

  /// Clear the display to white.
  pub fn clear(&mut self) -> Result<(), DisplayError> {
    let buf_len = (WIDTH as usize * HEIGHT as usize) / 2;
    // 0x11 = two white pixels (high nibble=1, low nibble=1)
    let buf = vec![0x11u8; buf_len];
    self.display_image(&buf)
  }

  /// Put the display into deep sleep mode for power saving.
  pub fn sleep(&mut self) -> Result<(), DisplayError> {
    self.send_command(0x10)?;
    self.send_data(&[0x01])?;
    thread::sleep(Duration::from_millis(100));
    self.power_off();
    log::info!("EPD entered deep sleep");
    Ok(())
  }
}

impl Drop for Epd7in3e {
  fn drop(&mut self) {
    self.power_off();
  }
}
