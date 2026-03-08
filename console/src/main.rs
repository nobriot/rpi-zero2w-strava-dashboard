use clap::{CommandFactory, FromArgMatches, Parser, builder::styling};
use std::io::{self, BufRead, BufReader, Write};

mod serial;
mod wizard;

const STYLES: styling::Styles = styling::Styles::styled()
    .header(styling::AnsiColor::Green.on_default().bold())
    .usage(styling::AnsiColor::Green.on_default().bold())
    .literal(styling::AnsiColor::Blue.on_default().bold())
    .placeholder(styling::AnsiColor::Cyan.on_default());

#[derive(Parser)]
#[command(
    name = "strava-console",
    about = "Setup console for Strava Dashboard on RPi"
)]
struct Args {
    /// Serial device path (auto-detected if not specified)
    #[arg(long)]
    device: Option<String>,

    /// Skip the guided wizard and go straight to REPL
    #[arg(long)]
    no_wizard: bool,
}

fn main() {
    env_logger::init();
    let mut matches = Args::command().styles(STYLES).term_width(80).get_matches();
    let args = Args::from_arg_matches_mut(&mut matches);

    let args = match args {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Incorrect arguments: {e}");
            std::process::exit(1);
        }
    };

    let device = match args.device.or_else(serial::auto_detect_device) {
        Some(d) => d,
        None => {
            eprintln!("Error: No USB serial device found.");
            eprintln!("Make sure the RPi is connected via USB and the USB daemon is running.");
            eprintln!("You can specify the device manually with --device /dev/ttyACM0");
            std::process::exit(1);
        }
    };

    eprint!("Connecting to {device}... ");
    let mut conn = match serial::Connection::open(&device) {
        Ok(c) => {
            eprintln!("✓");
            c
        }
        Err(e) => {
            eprintln!("✗");
            eprintln!("Failed to connect: {e}");
            std::process::exit(1);
        }
    };

    // Verify connection with a ping
    match conn.send(&protocol::Request::Ping) {
        Ok(resp) if resp.ok => {}
        _ => {
            eprintln!("Warning: Ping failed — daemon may not be running on the device.");
        }
    }

    if !args.no_wizard {
        wizard::run(&mut conn);
    }

    eprintln!("\nType 'help' for available commands, or 'quit' to exit.\n");
    run_repl(&mut conn);
}

fn run_repl(conn: &mut serial::Connection) {
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin.lock());

    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {}
            Err(e) => {
                eprintln!("Read error: {e}");
                break;
            }
        }

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        match line {
            "quit" | "exit" | "q" => break,
            "help" | "h" | "?" => print_help(),
            _ => handle_command(conn, line),
        }
    }

    eprintln!("Bye!");
}

fn handle_command(conn: &mut serial::Connection, input: &str) {
    let parts: Vec<&str> = input.splitn(3, ' ').collect();
    let cmd = parts[0];
    let arg1 = parts.get(1).copied().unwrap_or("");
    let arg2 = parts.get(2).copied().unwrap_or("");

    let request = match cmd {
        "ping" => Some(protocol::Request::Ping),
        "status" => Some(protocol::Request::Status),
        "refresh" => Some(protocol::Request::DashboardRefresh),

        "wifi" => match arg1 {
            "status" | "" => Some(protocol::Request::WifiStatus),
            "scan" => Some(protocol::Request::WifiScan),
            "add" => {
                if arg2.is_empty() {
                    wifi_add_interactive(conn);
                    return;
                }
                // wifi add <ssid> — prompt for password
                let password = prompt_password(&format!("Password for '{arg2}': "));
                Some(protocol::Request::WifiAdd {
                    ssid: arg2.to_string(),
                    password,
                })
            }
            "forget" => {
                if arg2.is_empty() {
                    eprintln!("Usage: wifi forget <ssid>");
                    return;
                }
                Some(protocol::Request::WifiForget {
                    ssid: arg2.to_string(),
                })
            }
            _ => {
                eprintln!("Unknown wifi subcommand: {arg1}");
                eprintln!("Usage: wifi [status|scan|add|forget]");
                None
            }
        },

        "config" => match arg1 {
            "show" | "" => Some(protocol::Request::ConfigGet),
            "push" => {
                if arg2.is_empty() {
                    eprintln!("Usage: config push <path_to_config.toml>");
                    return;
                }
                match std::fs::read_to_string(arg2) {
                    Ok(content) => Some(protocol::Request::ConfigPush { toml: content }),
                    Err(e) => {
                        eprintln!("Failed to read file: {e}");
                        return;
                    }
                }
            }
            _ => {
                eprintln!("Unknown config subcommand: {arg1}");
                eprintln!("Usage: config [show|push <file>]");
                None
            }
        },

        "auth" => {
            run_auth_command(conn);
            return;
        }

        _ => {
            eprintln!("Unknown command: {cmd}. Type 'help' for available commands.");
            None
        }
    };

    if let Some(req) = request {
        match conn.send(&req) {
            Ok(resp) => print_response(&resp),
            Err(e) => eprintln!("Communication error: {e}"),
        }
    }
}

fn wifi_add_interactive(conn: &mut serial::Connection) {
    // Scan for networks first
    eprintln!("Scanning for WiFi networks...");
    match conn.send(&protocol::Request::WifiScan) {
        Ok(resp) if resp.ok => {
            if let Some(protocol::ResponseData::WifiNetworks(networks)) = &resp.data {
                if networks.is_empty() {
                    eprintln!("No networks found.");
                    return;
                }
                for (i, net) in networks.iter().enumerate() {
                    eprintln!(
                        "  {}. {} ({} dBm) [{}]",
                        i + 1,
                        net.ssid,
                        net.signal,
                        net.security
                    );
                }

                let choice = prompt("Select network (number): ");
                let idx: usize = match choice.parse::<usize>() {
                    Ok(n) if n >= 1 && n <= networks.len() => n - 1,
                    _ => {
                        eprintln!("Invalid selection.");
                        return;
                    }
                };

                let ssid = &networks[idx].ssid;
                let password = prompt_password(&format!("Password for '{}': ", ssid));

                eprint!("Connecting... ");
                match conn.send(&protocol::Request::WifiAdd {
                    ssid: ssid.clone(),
                    password,
                }) {
                    Ok(resp) if resp.ok => eprintln!("✓ Connected"),
                    Ok(resp) => eprintln!("✗ {}", resp.error.unwrap_or_default()),
                    Err(e) => eprintln!("✗ {e}"),
                }
            }
        }
        Ok(resp) => eprintln!("Scan failed: {}", resp.error.unwrap_or_default()),
        Err(e) => eprintln!("Communication error: {e}"),
    }
}

fn run_auth_command(conn: &mut serial::Connection) {
    // Get current config from device to read client_id / client_secret
    eprint!("Reading config from device... ");
    let config_toml = match conn.send(&protocol::Request::ConfigGet) {
        Ok(resp) if resp.ok => {
            if let Some(protocol::ResponseData::Text(toml)) = resp.data {
                eprintln!("✓");
                toml
            } else {
                eprintln!("✗ No config data");
                return;
            }
        }
        Ok(resp) => {
            eprintln!("✗ {}", resp.error.unwrap_or_default());
            return;
        }
        Err(e) => {
            eprintln!("✗ {e}");
            return;
        }
    };

    // Parse the config to get client_id and client_secret
    let config = match strava::config::Config::from_toml(&config_toml) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Invalid config on device: {e}");
            eprintln!("Please push a valid config first: config push <file>");
            return;
        }
    };

    // Run OAuth flow locally (opens browser on this machine)
    eprintln!("Starting OAuth authorization flow...");
    eprintln!("A browser window will open for Strava authorization.");
    match strava::oauth::run_auth_flow(&config) {
        Ok(token_response) => {
            eprintln!("Authorization successful!");

            // Update config with new refresh token and push back
            let mut config = config;
            config.set_refresh_token(token_response.refresh_token);
            let updated_toml = match config.to_toml() {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("Failed to serialize config: {e}");
                    return;
                }
            };

            eprint!("Pushing updated config to device... ");
            match conn.send(&protocol::Request::ConfigPush { toml: updated_toml }) {
                Ok(resp) if resp.ok => eprintln!("✓"),
                Ok(resp) => eprintln!("✗ {}", resp.error.unwrap_or_default()),
                Err(e) => eprintln!("✗ {e}"),
            }
        }
        Err(e) => {
            eprintln!("Authorization failed: {e}");
        }
    }
}

fn print_response(resp: &protocol::Response) {
    if resp.ok {
        match &resp.data {
            Some(protocol::ResponseData::Text(s)) => eprintln!("{s}"),
            Some(protocol::ResponseData::WifiStatus(info)) => {
                if info.connected {
                    eprintln!(
                        "WiFi: Connected to {} ({})",
                        info.ssid.as_deref().unwrap_or("?"),
                        info.ip.as_deref().unwrap_or("no IP")
                    );
                } else {
                    eprintln!("WiFi: Not connected");
                }
            }
            Some(protocol::ResponseData::WifiNetworks(networks)) => {
                if networks.is_empty() {
                    eprintln!("No networks found.");
                } else {
                    for net in networks {
                        eprintln!("  {} ({} dBm) [{}]", net.ssid, net.signal, net.security);
                    }
                }
            }
            Some(protocol::ResponseData::SystemStatus(status)) => {
                eprintln!(
                    "  WiFi:    {} {}",
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
                    "  Config:  {}",
                    if status.config_valid {
                        "✓ Valid"
                    } else {
                        "✗ Missing or invalid"
                    }
                );
                eprintln!(
                    "  Auth:    {}",
                    if status.auth_valid {
                        "✓ Valid"
                    } else {
                        "✗ Invalid or missing token"
                    }
                );
                if let Some(pct) = status.battery_pct {
                    eprintln!("  Battery: {}%", pct);
                }
            }
            None => eprintln!("OK"),
        }
    } else {
        eprintln!(
            "Error: {}",
            resp.error.as_deref().unwrap_or("unknown error")
        );
    }
}

fn prompt(msg: &str) -> String {
    eprint!("{msg}");
    io::stderr().flush().unwrap();
    let mut s = String::new();
    io::stdin().read_line(&mut s).unwrap();
    s.trim().to_string()
}

fn prompt_password(msg: &str) -> String {
    // Simple password prompt (no echo hiding for portability)
    prompt(msg)
}

fn print_help() {
    eprintln!(
        "\
Commands:
  status              Show system status (WiFi, config, auth, battery)
  wifi                Show WiFi connection status
  wifi scan           Scan for available networks
  wifi add [ssid]     Connect to a WiFi network (interactive if no ssid given)
  wifi forget <ssid>  Forget a saved network
  config show         Show current config.toml on device
  config push <file>  Push a config.toml file to device
  auth                Run Strava OAuth flow and push token to device
  refresh             Trigger dashboard refresh (clears cache)
  ping                Test connection to device
  help                Show this help
  quit                Exit console"
    );
}
