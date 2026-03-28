# WaveShare RPI Zero 2W Strava Dashboard

A Strava dashboard for
[WaveShare Raspberry Pi Zero 2W Photo Painter](https://www.waveshare.com/wiki/RPi_Zero_PhotoPainter)
with e-paper display.

![Rust](https://img.shields.io/badge/rust-stable-orange.svg)
![License](https://img.shields.io/badge/license-MIT-blue.svg)

---

## Quick demo

TODO:Insert a picture

## RTC Notes:

config.txt:

```ini
dtoverlay=i2c-rtc,ds3231,wakeup-source
dtparam=i2c_arm=on
```

Sync time: 

```bash
sudo hwclock -w --utc
```

TPL5110 timer ? 
Seems like a good possibility.

## TODOs

Witty Pi 4 Mini ?
Seems overkill
https://www.digikey.dk/da/products/detail/pimoroni-ltd/witty-pi-4-mini/16716803


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

Those displays come with **Raspberry Pi OS** pre-flashed, and some repos
configured. The RPI has the default password and SSH enabled.

Those bandits use NetworkManager

TODO: Explain here how to add a nmconnection file to add your WiFi

### 2. Setup your Strava API Tokens


TODO: Write me
Run the dashboard on the dev machine with --auth. It will generate a config
file that you have to copy over on the rpi.

### 2. Build & Deploy

On your dev machine:

```bash
# Install cross-compilation tool (once)
cargo install cross

# Build all binaries for RPi
cross build --release --target aarch64-unknown-linux-gnu

# Copy binaries to RPi
scp target/aarch64-unknown-linux-gnu/release/rpi-zero2w-strava-dash pi@<host>:/usr/local/bin/

# Copy systemd services
scp install/strava-dashboard.service pi@<host>:/etc/systemd/system/

# Enable services on the RPi (via SSH)
ssh pi@<host> 'sudo systemctl daemon-reload && sudo systemctl enable --now strava-dashboard'
```

or use just: 

```bash
just dev
```

Then run the strava auth - to allow your application to pull your data:


```bash
just strava-auth
```

Finally, deploy to the RPi:

```bash
just deploy pi@<host>
just deploy-config <config.toml>
```

---

## Architecture

Cargo workspace with 3 crates:

| Crate | Type | Runs on | Purpose |
|-------|------|---------|---------|
| **`strava`** | library | — | Strava API client, OAuth, caching, stats |
| **`display`** | library | — | E-paper renderer, hardware drivers (SPI, I2C) |
| **`dashboard`** | binary | RPi | Main loop: fetch → render → display |

---

## Configuration

See the `config.example.toml` for available keys.
Config file (default): `~/.config/rpi-zero2w-strava-dash/config.toml`

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

## Credits

Totally inspired by [Ibis Dash](https://github.com/ibisette/Ibis_Dash_Esp32s3_PhotoPainter).

See also: [Statistics-for-Strava](https://github.com/robiningelbrecht/statistics-for-strava).
