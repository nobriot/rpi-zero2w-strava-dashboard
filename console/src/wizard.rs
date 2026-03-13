//! Guided setup wizard — auto-detects what needs configuration and walks through it.

use crate::serial::Connection;

pub fn run(conn: &mut Connection) {
    eprintln!("\n=== Strava Dashboard Setup ===\n");
    eprint!("Checking system status... ");

    let status = match conn.send(&protocol::Request::Status) {
        Ok(resp) if resp.ok => {
            if let Some(protocol::ResponseData::SystemStatus(s)) = resp.data {
                eprintln!("done\n");
                s
            } else {
                eprintln!("unexpected response");
                return;
            }
        }
        Ok(resp) => {
            eprintln!("failed: {}", resp.error.unwrap_or_default());
            return;
        }
        Err(e) => {
            eprintln!("failed: {e}");
            return;
        }
    };

    // Show current state
    eprintln!(
        "  WiFi:   {} {}",
        if status.wifi_connected { "✓" } else { "✗" },
        status
            .wifi_ssid
            .as_deref()
            .unwrap_or(if status.wifi_connected {
                "Connected"
            } else {
                "Not connected"
            })
    );
    eprintln!(
        "  Config: {}",
        if status.config_valid {
            "✓ Valid"
        } else {
            "✗ Missing or invalid"
        }
    );
    eprintln!(
        "  Auth:   {}",
        if status.auth_valid {
            "✓ Valid"
        } else {
            "✗ Invalid or missing token"
        }
    );

    let needs_wifi = !status.wifi_connected;
    let needs_config = !status.config_valid;
    let needs_auth = !status.auth_valid;

    if !needs_wifi && !needs_config && !needs_auth {
        eprintln!("\n✓ Everything is configured! Dashboard should be running.");
        return;
    }

    let total_steps = needs_wifi as u32 + needs_config as u32 + needs_auth as u32;
    let mut step = 0u32;

    // Step: WiFi
    if needs_wifi {
        step += 1;
        eprintln!("\nStep {step}/{total_steps}: WiFi Setup");
        setup_wifi(conn);
    }

    // Step: Config
    if needs_config {
        step += 1;
        eprintln!("\nStep {step}/{total_steps}: Strava API Credentials");
        setup_config(conn);
    }

    // Step: Auth
    if needs_auth {
        step += 1;
        eprintln!("\nStep {step}/{total_steps}: Strava Authorization");
        crate::run_auth_command(conn);
    }

    // Verify
    eprintln!("\nVerifying setup...");
    match conn.send(&protocol::Request::Status) {
        Ok(resp) if resp.ok => {
            if let Some(protocol::ResponseData::SystemStatus(s)) = resp.data {
                let all_ok = s.wifi_connected && s.config_valid && s.auth_valid;
                if all_ok {
                    eprintln!("✓ All set! Triggering first dashboard refresh...");
                    match conn.send(&protocol::Request::DashboardRefresh) {
                        Ok(r) if r.ok => eprintln!("✓ Dashboard refresh triggered."),
                        _ => eprintln!(
                            "Note: Could not trigger refresh. It will happen on next cycle."
                        ),
                    }
                } else {
                    eprintln!("Some items may still need attention. Use the REPL commands below.");
                }
            }
        }
        _ => eprintln!("Could not verify. Use 'status' command to check."),
    }
}

fn setup_wifi(conn: &mut Connection) {
    crate::wifi_add_interactive(conn);
}

fn setup_config(conn: &mut Connection) {
    eprintln!("You need to provide your Strava API credentials.");
    eprintln!("Create an app at https://www.strava.com/settings/api\n");

    let client_id = crate::prompt("  Client ID: ");
    let client_secret = crate::prompt("  Client Secret: ");

    let toml_content = format!(
        r#"# Strava API credentials
[strava]
client_id = "{client_id}"
client_secret = "{client_secret}"
refresh_token = "PLACEHOLDER"

# Display settings (defaults)
[display]
sleep_interval_secs = 10800
quiet_start_hour = 20
quiet_end_hour = 8

[[display.goals]]
sport = "run"
km = 800.0

[[display.goals]]
sport = "ride"
km = 5000.0

[[display.goals]]
sport = "swim"
km = 30.0
"#
    );

    eprint!("  Pushing config to device... ");
    match conn.send(&protocol::Request::ConfigPush { toml: toml_content }) {
        Ok(resp) if resp.ok => eprintln!("✓"),
        Ok(resp) => eprintln!("✗ {}", resp.error.unwrap_or_default()),
        Err(e) => eprintln!("✗ {e}"),
    }
}
