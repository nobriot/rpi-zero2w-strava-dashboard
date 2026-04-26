# WaveShare e-paper RPI Zero 2W Strava Dashboard

A Strava dashboard for
[WaveShare Raspberry Pi Zero 2W Photo Painter](https://www.waveshare.com/wiki/RPi_Zero_PhotoPainter)
with e-paper display.

![Rust](https://img.shields.io/badge/rust-stable-orange.svg)
![License](https://img.shields.io/badge/license-MIT-blue.svg)

Somebody said vide-coding is for toys, not production...

So let's build a toy! 🙂

This is the first project where I experimented with copilot / claude a little
bit to get a feel for what they can do.

Started coding the thing myself and slowly let the coding agent take over,
though I probably micro-manage them too much.

---

## Quick Start

On your dev machine:

```bash
# Install cross-compilation tool (once)
cargo install cross

# Build all binaries for RPi
cross build --release --target aarch64-unknown-linux-gnu

# Copy binaries to RPi
scp target/aarch64-unknown-linux-gnu/release/rpi-zero2w-strava-dashboard pi@<host>:/usr/local/bin/

# Copy systemd services
scp dist/strava-dashboard.service pi@<host>:/etc/systemd/system/

# Enable services on the RPi (via SSH)
ssh pi@<host> 'sudo systemctl daemon-reload && sudo systemctl enable --now strava-dashboard'
```

Install [just](https://just.systems/man/en/), and get started with:

```bash
just dev
```

Then run the strava auth - to allow your application to pull your data:

```bash
just strava-auth
```

Finally, deploy to the RPi if you want to run it on one:

```bash
just deploy pi@<host> <config.toml>
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

## Credits

Totally inspired by [Ibis Dash](https://github.com/ibisette/Ibis_Dash_Esp32s3_PhotoPainter).
See also: [Statistics-for-Strava](https://github.com/robiningelbrecht/statistics-for-strava).

