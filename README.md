# RPi Zero 2W Strava Dash

A Strava dashboard for Raspberry Pi Zero 2W with Waveshare PhotoPainter
e-paper display.

![Rust](https://img.shields.io/badge/rust-stable-orange.svg)
![License](https://img.shields.io/badge/license-MIT-blue.svg)

---

## What It Does

Displays your Strava stats on a beautiful 7.5" e-paper display:
- **Distance, time, and activity count**
- **Last activity details with route visualization**
- **Progress toward your yearly goal**
- **Auto-refresh** at configurable intervals
- **Low power consumption** - perfect for always-on display

---

## 🛠️ Hardware Requirements

- **Raspberry Pi Zero 2W** (or any RPi with SPI)
- **Waveshare 7.5" ACeP 7-Color E-Paper Display** (IT8951 controller)
- **MicroSD card** (16GB+ recommended)
- **Power supply** (5V 2.5A recommended) or battery

---

## Software Prerequisites

1. **Raspberry Pi OS** (64-bit recommended)
2. **Rust** (stable toolchain, 1.93+)
3. **[cross](https://github.com/cross-rs/cross)** (for cross-compilation only)

### Install Rust on your dev machine:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### Install cross-compilation tools:

```bash
rustup target add aarch64-unknown-linux-gnu
cargo install cross
```

---

##  Installation

### Clone the Repository

```bash
git clone https://github.com/nobriot/rpi-zero2w-strava-dash.git
cd rpi-zero2w-strava-dash
```

### Get a Strava token

Get the auth flow

### Copy the generated configuration over to the RPi

```bash
cp config.example.toml config.toml
nano config.toml
```

### Build for RPi Zero 2w

```bash
# Cross-compile for Raspberry Pi Zero 2W (requires cross)
cross build --release --target aarch64-unknown-linux-gnu
```

### Deploy to Raspberry Pi

```bash
scp target/aarch64-unknown-linux-gnu/release/dashboard pi@<host>:~/
```

### Run

```bash
sudo ./target/release/rpi-zero2w-strava-dash
```

> **Note:** `sudo` is required for GPIO/SPI access

---

## What it looks like

TODO: Insert picture when I have one


It is possible to generate a PNG of what the frame looks like:

```bash
rpi-zero2w-strava-dash --save-png frame.png
```


---

## TODOs

- [ ] USB setup for the RPi ? (just install the USB listener, then connect, then we configure from the dev machine?)
- [ ] Complete IT8951 display driver implementation
- [ ] Advanced graphics rendering with fonts
- [ ] systemd service setup

---

## Credits

Inspired by [Ibis Dash](https://github.com/ibisette/Ibis_Dash_Esp32s3_PhotoPainter)
