//! Shared protocol types for USB serial communication between the RPi daemon
//! and the dev-machine console.
//!
//! Wire format: nd-JSON (one JSON object per line).

use serde::{Deserialize, Serialize};

// ── Requests (console → daemon) ──────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Request {
  Ping,
  WifiStatus,
  WifiScan,
  WifiAdd { ssid: String, password: String },
  WifiForget { ssid: String },
  ConfigGet,
  ConfigPush { toml: String },
  Status,
  DashboardRefresh,
}

// ── Responses (daemon → console) ──────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
  pub ok:    bool,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub data:  Option<ResponseData>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResponseData {
  Text(String),
  WifiStatus(WifiStatusInfo),
  WifiNetworks(Vec<WifiNetwork>),
  SystemStatus(SystemStatus),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WifiStatusInfo {
  pub connected: bool,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub ssid:      Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub ip:        Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WifiNetwork {
  pub ssid:     String,
  pub signal:   i32,
  pub security: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
  pub wifi_connected: bool,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub wifi_ssid:      Option<String>,
  pub config_valid:   bool,
  pub auth_valid:     bool,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub battery_pct:    Option<u8>,
}

// ── Helpers ──────────────────────────────────────────────────────────

impl Response {
  pub fn ok_empty() -> Self {
    Self { ok:    true,
           data:  None,
           error: None, }
  }

  pub fn ok_text(text: impl Into<String>) -> Self {
    Self { ok:    true,
           data:  Some(ResponseData::Text(text.into())),
           error: None, }
  }

  pub fn ok_data(data: ResponseData) -> Self {
    Self { ok:    true,
           data:  Some(data),
           error: None, }
  }

  pub fn err(msg: impl Into<String>) -> Self {
    Self { ok:    false,
           data:  None,
           error: Some(msg.into()), }
  }
}

/// Serialize a request to a JSON line (with trailing newline).
pub fn encode_request(req: &Request) -> String {
  let mut s = serde_json::to_string(req).expect("failed to serialize request");
  s.push('\n');
  s
}

/// Serialize a response to a JSON line (with trailing newline).
pub fn encode_response(resp: &Response) -> String {
  let mut s = serde_json::to_string(resp).expect("failed to serialize response");
  s.push('\n');
  s
}

/// Deserialize a request from a JSON line.
pub fn decode_request(line: &str) -> Result<Request, serde_json::Error> {
  serde_json::from_str(line.trim())
}

/// Deserialize a response from a JSON line.
pub fn decode_response(line: &str) -> Result<Response, serde_json::Error> {
  serde_json::from_str(line.trim())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn round_trip_ping() {
    let req = Request::Ping;
    let encoded = encode_request(&req);
    let decoded = decode_request(&encoded).unwrap();
    assert!(matches!(decoded, Request::Ping));
  }

  #[test]
  fn round_trip_wifi_add() {
    let req = Request::WifiAdd { ssid:     "TestNet".into(),
                                 password: "pass123".into(), };
    let encoded = encode_request(&req);
    let decoded = decode_request(&encoded).unwrap();
    match decoded {
      Request::WifiAdd { ssid, password } => {
        assert_eq!(ssid, "TestNet");
        assert_eq!(password, "pass123");
      },
      _ => panic!("wrong variant"),
    }
  }

  #[test]
  fn round_trip_response() {
    let resp =
      Response::ok_data(ResponseData::WifiStatus(WifiStatusInfo { connected: true,
                                                                  ssid:      Some("MyNet".into()),
                                                                  ip:
                                                                    Some("192.168.1.42".into()), }));
    let encoded = encode_response(&resp);
    let decoded = decode_response(&encoded).unwrap();
    assert!(decoded.ok);
  }

  #[test]
  fn error_response() {
    let resp = Response::err("something went wrong");
    let encoded = encode_response(&resp);
    let decoded = decode_response(&encoded).unwrap();
    assert!(!decoded.ok);
    assert_eq!(decoded.error.unwrap(), "something went wrong");
  }
}
