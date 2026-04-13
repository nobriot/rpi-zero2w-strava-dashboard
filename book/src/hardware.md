# Hardware Details

This chapter provides technical details about the PhotoPainter hardware for
reference. You don't need to read this to set up the dashboard, but it may be
helpful for debugging or understanding how things work.

## Waveshare RPi Zero PhotoPainter

The PhotoPainter is a carrier board designed for the Raspberry Pi Zero 2W. It
integrates several components:

| Component | Function |
|-----------|----------|
| **7.3" ACeP e-Paper display** | The main screen |
| **INA219** | Battery voltage/current monitor |
| **DS3231** | Real-time clock (battery-backed) |
| **UPS circuit** | Battery charger and power management |
| **Photo frame** | Enclosure |

## E-Paper display

| Specification | Value |
|--------------|-------|
| Size | 7.3 inches (diagonal) |
| Resolution | 800 x 480 pixels |
| Colors | 6: Black, White, Red, Yellow, Green, Blue |
| Interface | SPI (Serial Peripheral Interface) |
| Refresh time | ~12 seconds |
| Power consumption | Very low (only uses power during refresh) |

The display retains its image with **zero power**. Once an image is drawn, it
stays visible even if the Pi is completely turned off.

### Pin connections

These are specific to the PhotoPainter board and differ from the standard
Waveshare e-Paper HAT.

| Signal | GPIO Pin | Purpose |
|--------|----------|---------|
| DIN | BCM 10 (MOSI) | Data sent to display |
| CLK | BCM 11 (SCLK) | Clock signal |
| CS | BCM 8 (CE0) | Chip select |
| DC | BCM 25 | Tells display if data is a command or image |
| RST | BCM 17 | Resets the display |
| BUSY | BCM 24 | Display signals when refresh is complete |
| PWR | BCM 27 | Turns display power on/off |

## Battery monitor (INA219)

The INA219 chip monitors the battery over I2C (address `0x43`). It measures:

- **Voltage** --- how full the battery is
- **Current** --- whether the battery is charging or discharging

### Battery level table

| Voltage | Approximate charge |
|---------|--------------------|
| 4.20 V | 100% |
| 4.06 V | 90% |
| 3.98 V | 80% |
| 3.92 V | 70% |
| 3.87 V | 60% |
| 3.82 V | 50% |
| 3.79 V | 40% |
| 3.77 V | 30% |
| 3.74 V | 20% |
| 3.68 V | 10% |
| 3.45 V | 0% |

The battery percentage is shown in the top-right corner of the dashboard.

## Power consumption

| State | Approximate draw |
|-------|-----------------|
| Active (WiFi + display refresh) | ~120 mA |
| Idle (WiFi on, display static) | ~100 mA |
| Low-power sleep (WiFi + HDMI off) | ~50-60 mA |

With a typical 3000 mAh battery and low-power mode enabled, the dashboard can
run for roughly 2-3 days on a single charge, depending on refresh frequency.

## References

- [Waveshare RPi Zero PhotoPainter Wiki](https://www.waveshare.com/wiki/RPi_Zero_PhotoPainter)
- [Waveshare 7.3" ACeP e-Paper Specs](https://www.waveshare.com/7.3inch-e-paper-hat-f.htm)
- [RPi boot config](https://www.raspberrypi.com/documentation/computers/config_txt.html)
- [Raspi_onoff](https://github.com/decodeais/Raspi_onoff)
- [RPI RTC DS3231](http://www.intellamech.com/RaspberryPi-projects/rpi_RTCds3231)
- [raspberry pi firmware](https://github.com/raspberrypi/firmware/blob/master/boot/overlays/README)
