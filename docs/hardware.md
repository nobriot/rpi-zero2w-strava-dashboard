# Hardware Reference

This document covers the Waveshare RPi Zero PhotoPainter board — a Raspberry Pi Zero 2W carrier board with a 7.3" 6-color ACeP e-Paper display, UPS battery, and RTC.

## Display: Waveshare 7.3" ACeP e-Paper (epd7in3e)

- **Resolution**: 800 × 480
- **Colors**: 6 (Black, White, Green, Blue, Red, Yellow)
- **Refresh time**: ~12 seconds (full refresh)
- **Interface**: SPI
- **Wire format**: 4 bits per pixel, 2 pixels packed per byte (high nibble = left pixel)
- **Color codes**: 0=Black, 1=White, 2=Green, 3=Blue, 4=Red, 5=Yellow
- **Buffer size**: 800 × 480 / 2 = 192,000 bytes

### Pin Mapping (RPi Zero PhotoPainter)

> ⚠️ These pin assignments differ from the standard Waveshare e-Paper HAT!

| Signal | BCM GPIO | Physical Pin | Direction |
|--------|----------|-------------|-----------|
| DIN    | BCM 10 (MOSI) | 19    | Pi → EPD  |
| CLK    | BCM 11 (SCLK) | 23    | Pi → EPD  |
| CS     | BCM 8 (CE0)   | 24    | Pi → EPD  |
| DC     | BCM 25        | 22    | Pi → EPD  |
| RST    | BCM 17        | 11    | Pi → EPD  |
| BUSY   | BCM 24        | 18    | EPD → Pi  |
| PWR    | BCM 27        | 13    | Pi → EPD  |

The **PWR** pin is unique to the PhotoPainter board — it controls the display power supply and must be driven HIGH before any SPI communication.

## UPS Battery Monitor: INA219

The board includes an INA219 current/voltage sensor on the I2C bus for battery monitoring.

- **I2C address**: `0x43`
- **Bus voltage register**: `0x02` (bits 15:3, LSB = 4 mV)
- **Current register**: `0x04`

### Battery Percentage (LiPo approximation)

| Voltage | Percentage |
|---------|-----------|
| ≥ 4.20V | 100%     |
| 4.06V   | 90%      |
| 3.98V   | 80%      |
| 3.92V   | 70%      |
| 3.87V   | 60%      |
| 3.82V   | 50%      |
| 3.79V   | 40%      |
| 3.77V   | 30%      |
| 3.74V   | 20%      |
| 3.68V   | 10%      |
| ≤ 3.45V | 0%       |

## RTC: DS3231 (Optional)

The board has a DS3231 real-time clock on I2C.
Not currently used by the dashboard — the Pi syncs time via NTP when connected to WiFi.

## Enabling SPI and I2C

On the Raspberry Pi, enable the required interfaces:

```bash
sudo raspi-config
# → Interface Options → SPI → Enable
# → Interface Options → I2C → Enable

# Or edit /boot/config.txt directly:
# dtparam=spi=on
# dtparam=i2c_arm=on

# Reboot after changes
sudo reboot
```

Verify the interfaces are available:

```bash
ls /dev/spidev0.0   # SPI
ls /dev/i2c-1        # I2C
```

## Cross-Compile and Deploy

Build on your development machine:

```bash
# Cross-compile for the RPi (aarch64)
cargo cross-build

# Copy the binary to the Pi
scp target/aarch64-unknown-linux-gnu/release/dashboard pi@<PI_IP>:~/
```

Run on the Pi:

```bash
# First run — set up Strava credentials
./dashboard --auth

# Normal operation (loops with 3h sleep)
RUST_LOG=info ./dashboard

# Single cycle for testing
RUST_LOG=info ./dashboard --once

# Save a PNG preview without the display
RUST_LOG=info ./dashboard --once --save-png preview.png
```

## References

- [Waveshare RPi Zero PhotoPainter Wiki](https://www.waveshare.com/wiki/RPi_Zero_PhotoPainter)
- [Waveshare 7.3" ACeP e-Paper Spec](https://www.waveshare.com/7.3inch-e-paper-hat-f.htm)
- [Ibis Dash (ESP32 reference)](https://github.com/ibisette/Ibis_Dash_Esp32s3_PhotoPainter)
