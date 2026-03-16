//! Serial port connection to the RPi USB daemon.

use std::io::{BufRead, BufReader, Write};
use std::time::Duration;

pub struct Connection {
  port: Box<dyn serialport::SerialPort>,
}

impl Connection {
  pub fn open(device: &str) -> Result<Self, String> {
    let port =
      serialport::new(device, 115200).timeout(Duration::from_secs(30))
                                     .open()
                                     .map_err(|e| format!("Failed to open {device}: {e}"))?;

    Ok(Self { port })
  }

  /// Send a request and wait for a response.
  pub fn send(&mut self, request: &protocol::Request) -> Result<protocol::Response, String> {
    let data = protocol::encode_request(request);
    self.port.write_all(data.as_bytes()).map_err(|e| format!("Write failed: {e}"))?;
    self.port.flush().map_err(|e| format!("Flush failed: {e}"))?;

    // Read one line of response
    let mut reader = BufReader::new(&mut self.port);
    let mut line = String::new();
    reader.read_line(&mut line).map_err(|e| format!("Read failed: {e}"))?;

    protocol::decode_response(&line).map_err(|e| format!("Invalid response: {e} — raw: {line}"))
  }
}

/// Try to auto-detect the USB serial device.
pub fn auto_detect_device() -> Option<String> {
  let candidates = if cfg!(target_os = "macos") {
    // macOS: /dev/tty.usbmodem*
    glob_devices("/dev/tty.usbmodem*")
  } else {
    // Linux: /dev/ttyACM*
    glob_devices("/dev/ttyACM*")
  };

  candidates.into_iter().next()
}

fn glob_devices(pattern: &str) -> Vec<String> {
  // Simple glob using std::fs — look for matching device files
  let dir = std::path::Path::new(pattern).parent().unwrap_or(std::path::Path::new("/dev"));
  let prefix = std::path::Path::new(pattern).file_name()
                                            .unwrap_or_default()
                                            .to_string_lossy()
                                            .replace('*', "");

  match std::fs::read_dir(dir) {
    Ok(entries) => entries.filter_map(|e| e.ok())
                          .filter(|e| e.file_name().to_string_lossy().starts_with(prefix.as_str()))
                          .map(|e| e.path().to_string_lossy().into_owned())
                          .collect(),
    Err(_) => Vec::new(),
  }
}
