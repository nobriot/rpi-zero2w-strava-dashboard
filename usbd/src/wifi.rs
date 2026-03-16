//! WiFi management via NetworkManager's `nmcli` command.

use protocol::{Response, ResponseData, WifiNetwork, WifiStatusInfo};
use std::process::Command;

/// Get current WiFi connection info.
pub fn get_wifi_info() -> WifiStatusInfo {
  let output = Command::new("nmcli").args(["-t",
                                           "-f",
                                           "GENERAL.STATE,GENERAL.CONNECTION,IP4.ADDRESS",
                                           "device",
                                           "show",
                                           "wlan0"])
                                    .output();

  match output {
    Ok(out) => {
      let text = String::from_utf8_lossy(&out.stdout);
      let connected = text.contains("connected");
      let ssid = parse_nmcli_field(&text, "GENERAL.CONNECTION:");
      let ip = parse_nmcli_field(&text, "IP4.ADDRESS[1]:");

      WifiStatusInfo { connected,
                       ssid: ssid.filter(|s| !s.is_empty() && s != "--"),
                       ip: ip.map(|s| s.split('/').next().unwrap_or(&s).to_string()) }
    },
    Err(_) => WifiStatusInfo { connected: false,
                               ssid:      None,
                               ip:        None, },
  }
}

pub fn wifi_status() -> Response {
  let info = get_wifi_info();
  Response::ok_data(ResponseData::WifiStatus(info))
}

pub fn wifi_scan() -> Response {
  // Trigger a rescan first
  let _ = Command::new("nmcli").args(["device", "wifi", "rescan"]).output();

  let output =
    Command::new("nmcli").args(["-t", "-f", "SSID,SIGNAL,SECURITY", "device", "wifi", "list"])
                         .output();

  match output {
    Ok(out) => {
      let text = String::from_utf8_lossy(&out.stdout);
      let mut networks: Vec<WifiNetwork> = Vec::new();
      let mut seen_ssids = std::collections::HashSet::new();

      for line in text.lines() {
        let parts: Vec<&str> = line.splitn(3, ':').collect();
        if parts.len() >= 3 {
          let ssid = parts[0].to_string();
          if ssid.is_empty() || !seen_ssids.insert(ssid.clone()) {
            continue;
          }
          let signal = parts[1].parse::<i32>().unwrap_or(0);
          let security = parts[2].to_string();
          networks.push(WifiNetwork { ssid, signal, security });
        }
      }

      // Sort by signal strength (strongest first)
      networks.sort_by(|a, b| b.signal.cmp(&a.signal));

      Response::ok_data(ResponseData::WifiNetworks(networks))
    },
    Err(e) => Response::err(format!("WiFi scan failed: {e}")),
  }
}

pub fn wifi_add(ssid: &str, password: &str) -> Response {
  let output = Command::new("nmcli").args(["device", "wifi", "connect", ssid, "password",
                                           password])
                                    .output();

  match output {
    Ok(out) => {
      if out.status.success() {
        Response::ok_empty()
      } else {
        let stderr = String::from_utf8_lossy(&out.stderr);
        Response::err(format!("Failed to connect: {stderr}"))
      }
    },
    Err(e) => Response::err(format!("Failed to run nmcli: {e}")),
  }
}

pub fn wifi_forget(ssid: &str) -> Response {
  let output = Command::new("nmcli").args(["connection", "delete", ssid]).output();

  match output {
    Ok(out) => {
      if out.status.success() {
        Response::ok_empty()
      } else {
        let stderr = String::from_utf8_lossy(&out.stderr);
        Response::err(format!("Failed to forget network: {stderr}"))
      }
    },
    Err(e) => Response::err(format!("Failed to run nmcli: {e}")),
  }
}

fn parse_nmcli_field(text: &str, field: &str) -> Option<String> {
  text.lines()
      .find(|line| line.starts_with(field))
      .and_then(|line| line.split_once(':'))
      .map(|(_, v)| v.trim().to_string())
}
