use clap::{CommandFactory, FromArgMatches, Parser, builder::styling};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};

mod wifi;

#[derive(Parser)]
#[command(name = "strava-usbd", about = "USB serial daemon for Strava dashboard")]
struct Args {
    /// Serial device path
    #[arg(long, default_value = "/dev/ttyGS0")]
    device: String,
}

const STYLES: styling::Styles = styling::Styles::styled()
    .header(styling::AnsiColor::Green.on_default().bold())
    .usage(styling::AnsiColor::Green.on_default().bold())
    .literal(styling::AnsiColor::Blue.on_default().bold())
    .placeholder(styling::AnsiColor::Cyan.on_default());

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

    log::info!("Starting USB daemon on {}", args.device);
    eprintln!("strava-usbd: listening on {}", args.device);

    loop {
        match run_session(&args.device) {
            Ok(()) => log::info!("Session ended, waiting for reconnect..."),
            Err(e) => log::error!("Session error: {e}, retrying in 2s..."),
        }
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
}

fn run_session(device: &str) -> Result<(), String> {
    let read_file =
        File::open(device).map_err(|e| format!("Failed to open {device} for reading: {e}"))?;
    let mut write_file = OpenOptions::new()
        .write(true)
        .open(device)
        .map_err(|e| format!("Failed to open {device} for writing: {e}"))?;

    let reader = BufReader::new(read_file);

    for line in reader.lines() {
        let line = line.map_err(|e| format!("Read error: {e}"))?;
        if line.trim().is_empty() {
            continue;
        }

        log::debug!("Received: {line}");

        let response = match protocol::decode_request(&line) {
            Ok(req) => handle_request(req),
            Err(e) => protocol::Response::err(format!("Invalid request: {e}")),
        };

        let out = protocol::encode_response(&response);
        write_file
            .write_all(out.as_bytes())
            .map_err(|e| format!("Write error: {e}"))?;
        write_file
            .flush()
            .map_err(|e| format!("Flush error: {e}"))?;
    }

    Ok(())
}

fn handle_request(req: protocol::Request) -> protocol::Response {
    match req {
        protocol::Request::Ping => protocol::Response::ok_text("pong"),

        protocol::Request::WifiStatus => wifi::wifi_status(),
        protocol::Request::WifiScan => wifi::wifi_scan(),
        protocol::Request::WifiAdd { ssid, password } => wifi::wifi_add(&ssid, &password),
        protocol::Request::WifiForget { ssid } => wifi::wifi_forget(&ssid),

        protocol::Request::ConfigGet => config_get(),
        protocol::Request::ConfigPush { toml } => config_push(&toml),

        protocol::Request::Status => system_status(),
        protocol::Request::DashboardRefresh => dashboard_refresh(),
    }
}

// ── Config commands ──────────────────────────────────────────────────

fn config_get() -> protocol::Response {
    let path = config_path();
    match std::fs::read_to_string(&path) {
        Ok(contents) => protocol::Response::ok_text(contents),
        Err(e) => protocol::Response::err(format!("Failed to read config: {e}")),
    }
}

fn config_push(toml_content: &str) -> protocol::Response {
    // Validate it's valid TOML that can parse as a Config
    if let Err(e) = toml::from_str::<toml::Value>(toml_content) {
        return protocol::Response::err(format!("Invalid TOML: {e}"));
    }

    let path = config_path();
    if let Some(parent) = std::path::Path::new(&path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    match std::fs::write(&path, toml_content) {
        Ok(()) => protocol::Response::ok_empty(),
        Err(e) => protocol::Response::err(format!("Failed to write config: {e}")),
    }
}

fn config_path() -> String {
    let dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from(".config"))
        .join("rpi-zero2w-strava-dash");
    dir.join("config.toml").to_string_lossy().into_owned()
}

// ── System status ────────────────────────────────────────────────────

fn system_status() -> protocol::Response {
    let wifi = wifi::get_wifi_info();

    // Check if config is valid
    let config_valid = strava::config::Config::load().is_ok();

    // Check if auth works (try to get a token)
    let auth_valid = if config_valid {
        match strava::config::Config::load() {
            Ok(config) => {
                let mut client = strava::client::Client::new(config);
                client.get_token().is_ok()
            }
            Err(_) => false,
        }
    } else {
        false
    };

    protocol::Response::ok_data(protocol::ResponseData::SystemStatus(
        protocol::SystemStatus {
            wifi_connected: wifi.connected,
            wifi_ssid: wifi.ssid,
            config_valid,
            auth_valid,
            battery_pct: None, // TODO: read from INA219
        },
    ))
}

// ── Dashboard refresh ────────────────────────────────────────────────

fn dashboard_refresh() -> protocol::Response {
    // Signal the dashboard to refresh by clearing the cache
    match strava::cache::Cache::new().clear() {
        Ok(()) => protocol::Response::ok_text("Cache cleared, dashboard will refresh next cycle"),
        Err(e) => protocol::Response::err(format!("Failed to clear cache: {e}")),
    }
}
