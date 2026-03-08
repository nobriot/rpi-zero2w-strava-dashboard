# RPi Zero 2W Strava Dash

A Strava dashboard for Raspberry Pi Zero 2W with Waveshare 7.3" e-paper display.

![Rust](https://img.shields.io/badge/rust-stable-orange.svg)
![License](https://img.shields.io/badge/license-MIT-blue.svg)

---

## What It Does

Displays your Strava stats on a 7.3" 6-color e-paper display:
- **Multi-sport tracking** — Run, Ride, Swim with yearly goal progress bars
- **Activity details** — Longest, fastest, race bests (5K/10K/HM)
- **Last activity with route polyline**
- **Auto-refresh** at configurable intervals with quiet hours
- **Low power** — battery monitoring via INA219
- **USB setup** — plug into your computer to configure WiFi and Strava auth

---

## Hardware Requirements

- **Raspberry Pi Zero 2W** (or any RPi with SPI + USB OTG)
- **Waveshare 7.3" ACeP 6-Color E-Paper Display** (800×480)
- **MicroSD card** (16GB+ recommended)
- **Power supply** (5V 2.5A) or UPS battery with INA219

---

## Quick Start

### 1. Prepare the RPi SD Card

Flash **Raspberry Pi OS** (64-bit, Bookworm) to an SD card. Boot the Pi and ensure SSH is enabled.

### 2. Set Up USB Gadget Mode

On the RPi (via SSH or directly):

```bash
sudo bash install/setup-usb-gadget.sh
sudo reboot
```

This configures the Pi as a USB serial gadget. After reboot, plugging the Pi into a computer via USB creates a serial port.

### 3. Build & Deploy

On your dev machine:

```bash
# Install cross-compilation tool (once)
cargo install cross

# Build all binaries for RPi
cross build --release --target aarch64-unknown-linux-gnu

# Copy binaries to RPi
scp target/aarch64-unknown-linux-gnu/release/rpi-zero2w-strava-dash pi@<host>:/usr/local/bin/
scp target/aarch64-unknown-linux-gnu/release/strava-usbd pi@<host>:/usr/local/bin/

# Copy systemd services
scp install/strava-dashboard.service pi@<host>:/etc/systemd/system/
scp install/strava-usbd.service pi@<host>:/etc/systemd/system/

# Enable services on the RPi (via SSH)
ssh pi@<host> 'sudo systemctl daemon-reload && sudo systemctl enable --now strava-usbd strava-dashboard'
```

### 4. Configure via USB Console

Plug the RPi into your computer via USB, then:

```bash
# Build the console tool for your dev machine
cargo build --release -p console

# Run the setup wizard
./target/release/strava-console
```

The wizard auto-detects the USB serial device and walks you through:
1. **WiFi setup** — scans and connects to a network
2. **Strava credentials** — enters your API client ID and secret
3. **OAuth authorization** — opens a browser to authorize, pushes tokens to Pi

Steps that are already configured are automatically skipped.

### 5. Console REPL

After the wizard, you drop into an interactive console:

```
> help
Commands:
  status              Show system status (WiFi, config, auth, battery)
  wifi                Show WiFi connection status
  wifi scan           Scan for available networks
  wifi add [ssid]     Connect to a WiFi network
  wifi forget <ssid>  Forget a saved network
  config show         Show current config.toml on device
  config push <file>  Push a config.toml file to device
  auth                Run Strava OAuth flow and push token to device
  refresh             Trigger dashboard refresh (clears cache)
  ping                Test connection to device
  help                Show this help
  quit                Exit console
```

---

## Architecture

Cargo workspace with 5 crates:

| Crate | Type | Runs on | Purpose |
|-------|------|---------|---------|
| **`strava`** | library | — | Strava API client, OAuth, caching, stats |
| **`display`** | library | — | E-paper renderer, hardware drivers (SPI, I2C) |
| **`protocol`** | library | — | Shared USB serial protocol types |
| **`dashboard`** | binary | RPi | Main loop: fetch → render → display |
| **`usbd`** | binary | RPi | USB serial daemon (WiFi, config management) |
| **`console`** | binary | Dev machine | Interactive setup and REPL |

### Data Flow

```
[Dev Machine]                    USB Serial                    [RPi Zero 2W]
strava-console  ←── JSON lines ──→  strava-usbd
                                         │
                                    config.toml
                                         │
                                    rpi-zero2w-strava-dash
                                    (dashboard binary)
                                         │
                                   ┌─────┴──────┐
                                   │             │
                              Strava API    E-paper Display
```

---

## Configuration

Config file: `~/.config/rpi-zero2w-strava-dash/config.toml`

```toml
# Strava API credentials
client_id = "YOUR_CLIENT_ID"
client_secret = "YOUR_CLIENT_SECRET"
refresh_token = "YOUR_REFRESH_TOKEN"

[display]
sleep_interval_secs = 10800  # 3 hours
quiet_start_hour = 20        # No refresh 20:00–08:00
quiet_end_hour = 8
run_goal_km = 2000.0
ride_goal_km = 5000.0
swim_goal_km = 200.0
```

The refresh token is automatically updated when it changes.

---

## CLI Arguments (dashboard)

```
--auth              Run Strava OAuth flow manually
--once              Run single cycle and exit
--save-png <path>   Save rendered dashboard as PNG
--clear-cache       Clear all cached data and exit
--show-all-sports   Show all sports even without activities (demo mode)
```

---

## Building & Testing

```bash
cargo build                          # Build dashboard (default member)
cargo build --workspace              # Build all crates
cargo test --workspace               # Run all tests
cargo test -p strava                 # Tests in the strava crate
RUST_LOG=debug cargo run -- --once   # Run once with debug logging
```

Requires Rust edition 2024 (rust-version 1.93+).

---

## Network Resilience

- **Auto token refresh**: On HTTP 401, the client automatically refreshes the access token and retries
- **Offline mode**: When network is unavailable, an offline screen is shown on the display
- **Cache**: API responses are cached locally with configurable TTL (3 hours default, 7 days for athlete data)

---

## Credits

Inspired by [Ibis Dash](https://github.com/ibisette/Ibis_Dash_Esp32s3_PhotoPainter)
