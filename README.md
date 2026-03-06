# 🦀 RPi Zero 2W Strava Dash

A Rust-based Strava dashboard for Raspberry Pi Zero 2W with Waveshare PhotoPainter e-paper display.

![Rust](https://img.shields.io/badge/rust-stable-orange.svg)
![License](https://img.shields.io/badge/license-MIT-blue.svg)

---

## 🪶 What It Does

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
- **Power supply** (5V 2.5A recommended)

---

## 📦 Software Prerequisites

1. **Raspberry Pi OS** (64-bit recommended)
2. **Rust** (stable toolchain)

### Install Rust on Raspberry Pi:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

---

## 🚀 Installation

### 1. Clone the Repository

```bash
git clone https://github.com/yourusername/rpi-zero2w-strava-dash.git
cd rpi-zero2w-strava-dash
```

### 2. Configure

```bash
cp config.example.toml config.toml
nano config.toml
```

Fill in your:
- WiFi credentials
- Strava API credentials (see [docs/STRAVA_API.md](docs/STRAVA_API.md))
- Display preferences

### 3. Build

```bash
cargo build --release
```

### 4. Run

```bash
sudo ./target/release/rpi-zero2w-strava-dash
```

> **Note:** `sudo` is required for GPIO/SPI access

---

## 🔧 Configuration

See `config.example.toml` for all available options:

- **WiFi**: Network credentials
- **Strava**: API credentials and sport type filter
- **Display**: Refresh interval, tracking period, goals
- **Hardware**: GPIO pin assignments

---

## 📚 Documentation

- [Hardware Setup Guide](docs/HARDWARE_SETUP.md)
- [Strava API Setup](docs/STRAVA_API.md)

---

## 🎯 Features

- ✅ Strava API integration with token refresh
- ✅ Multiple sport types (Run, Ride, Swim, Hike, Walk)
- ✅ Polyline route visualization
- ✅ Weekly/Monthly/Yearly tracking periods
- ✅ Goal progress tracking
- ✅ Low-power e-paper display
- ✅ Async Rust implementation
- ✅ Robust error handling

---

## 🔮 Roadmap

- [ ] Complete IT8951 display driver implementation
- [ ] Advanced graphics rendering with fonts
- [ ] Web interface for configuration
- [ ] Multiple activity type support on single screen
- [ ] Weather integration
- [ ] systemd service setup

---

## 🤝 Contributing

Contributions welcome! Please open an issue or PR.

---

## 📄 License

MIT License - see [LICENSE](LICENSE) file

---

## 🙏 Credits

Inspired by [Ibis Dash](https://github.com/ibisette/Ibis_Dash_Esp32s3_PhotoPainter) 🪶

Built with:
- [Tokio](https://tokio.rs/) - Async runtime
- [reqwest](https://github.com/seanmonstar/reqwest) - HTTP client
- [rppal](https://github.com/golemparts/rppal) - Raspberry Pi GPIO/SPI
- [embedded-graphics](https://github.com/embedded-graphics/embedded-graphics) - Graphics library

---

**Happy tracking!** 🏃‍♂️🦀
