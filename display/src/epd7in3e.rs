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
/// ACeP refresh takes ~30s; allow headroom.
const BUSY_TIMEOUT: Duration = Duration::from_secs(60);

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

  /// Initialize the epd7in3e ACeP display controller.
  /// Command sequence ported from Waveshare C reference driver (EPD_7in3e.c).
  fn init_display(&mut self) -> Result<(), DisplayError> {
    self.wait_busy()?;
    thread::sleep(Duration::from_millis(30));

    // CMDH
    self.send_command(0xAA)?;
    self.send_data(&[0x49, 0x55, 0x20, 0x08, 0x09, 0x18])?;

    self.send_command(0x01)?;
    self.send_data(&[0x3F])?;

    self.send_command(0x00)?;
    self.send_data(&[0x5F, 0x69])?;

    self.send_command(0x03)?;
    self.send_data(&[0x00, 0x54, 0x00, 0x44])?;

    self.send_command(0x05)?;
    self.send_data(&[0x40, 0x1F, 0x1F, 0x2C])?;

    self.send_command(0x06)?;
    self.send_data(&[0x6F, 0x1F, 0x17, 0x49])?;

    self.send_command(0x08)?;
    self.send_data(&[0x6F, 0x1F, 0x1F, 0x22])?;

    // PLL
    self.send_command(0x30)?;
    self.send_data(&[0x03])?;

    // VCOM
    self.send_command(0x50)?;
    self.send_data(&[0x3F])?;

    // TCON
    self.send_command(0x60)?;
    self.send_data(&[0x02, 0x00])?;

    // Resolution: 800x480
    self.send_command(0x61)?;
    self.send_data(&[0x03, 0x20, 0x01, 0xE0])?;

    self.send_command(0x84)?;
    self.send_data(&[0x01])?;

    self.send_command(0xE3)?;
    self.send_data(&[0x2F])?;

    // POWER_ON
    self.send_command(0x04)?;
    self.wait_busy()?;

    log::info!("EPD initialized ({}x{})", WIDTH, HEIGHT);
    Ok(())
  }

  /// Trigger display refresh after writing image data.
  /// Sequence from Waveshare C reference driver (EPD_7IN3E_TurnOnDisplay).
  fn turn_on_display(&mut self) -> Result<(), DisplayError> {
    // POWER_ON
    self.send_command(0x04)?;
    self.wait_busy()?;

    // Booster re-setting
    self.send_command(0x06)?;
    self.send_data(&[0x6F, 0x1F, 0x17, 0x49])?;

    // DISPLAY_REFRESH
    self.send_command(0x12)?;
    self.send_data(&[0x00])?;
    self.wait_busy()?;

    // POWER_OFF
    self.send_command(0x02)?;
    self.send_data(&[0x00])?;
    self.wait_busy()?;

    Ok(())
  }

  /// Display a 6-color packed image buffer.
  /// Buffer format: 2 pixels per byte (high nibble = left pixel, low nibble =
  /// right pixel). Colors: 0=Black, 1=White, 2=Yellow, 3=Red, 5=Blue,
  /// 6=Green. Expected length: WIDTH * HEIGHT / 2 = 192000 bytes.
  pub fn display_image(&mut self, buf: &[u8]) -> Result<(), DisplayError> {
    let expected_len = (WIDTH as usize * HEIGHT as usize) / 2;
    if buf.len() != expected_len {
      return Err(DisplayError::Render(format!("Image buffer size mismatch: expected {expected_len}, got {}",
                                              buf.len())));
    }

    log::info!("Sending image data to EPD...");

    // Write image data (command 0x10)
    self.send_command(0x10)?;
    self.send_data(buf)?;

    // Trigger display refresh
    self.turn_on_display()?;

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
    // POWER_OFF
    self.send_command(0x02)?;
    self.send_data(&[0x00])?;
    self.wait_busy()?;

    // DEEP_SLEEP
    self.send_command(0x07)?;
    self.send_data(&[0xA5])?;

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
