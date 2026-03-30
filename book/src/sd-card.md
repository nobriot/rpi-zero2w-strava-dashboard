# Preparing the SD Card

The PhotoPainter kit usually comes with a microSD card that has Raspberry Pi OS
pre-installed. If yours came with one, you can skip to the
[Enable SPI and I2C](#enable-spi-and-i2c) section.

If you need to flash a new card, follow the steps below.

## Flashing Raspberry Pi OS

1. Download and install the
   [Raspberry Pi Imager](https://www.raspberrypi.com/software/) on your
   computer.

2. Insert your microSD card into your computer.

3. Open Raspberry Pi Imager and select:
   - **Device:** Raspberry Pi Zero 2 W
   - **Operating System:** Raspberry Pi OS (64-bit) --- under "Raspberry Pi OS
     (other)", pick the **Lite** version (no desktop needed)
   - **Storage:** Your microSD card

4. Click **Next**, then **Edit Settings** to customize:
   - **Hostname:** `photopainter` (or any name you like)
   - **Username / Password:** Set a username and password (e.g. `pi` /
     `your-password`)
   - **WiFi:** Enter your WiFi network name and password (see
     [Setting Up WiFi](./wifi.md) for details)
   - **SSH:** Enable SSH with password authentication

5. Click **Save**, then **Yes** to write the image.

6. Wait for the write and verification to complete.

## Enable SPI and I2C

The e-paper display uses SPI and the battery monitor uses I2C. These need to
be enabled before the first boot.

After flashing, the SD card will appear as a drive called `bootfs` (or
`boot`). Open it in your file manager or terminal.

### Edit config.txt

Open the file `config.txt` (on newer images it's at `firmware/config.txt` or
just `config.txt` in the boot partition). Add or uncomment these lines:

```ini
# Enable SPI (for the e-paper display)
dtparam=spi=on

# Enable I2C (for the battery monitor and RTC)
dtparam=i2c_arm=on
```

### Optional: Enable the real-time clock

If you want the Pi to keep accurate time even without WiFi, add:

```ini
dtoverlay=i2c-rtc,ds3231,wakeup-source
```

This tells the Pi to use the DS3231 real-time clock chip on the PhotoPainter
board. After the first boot with internet access, sync the time:

```bash
sudo hwclock -w --utc
```

## Eject and insert

Safely eject the SD card from your computer, insert it into the Raspberry Pi
Zero 2W, and power it on.

The first boot takes a minute or two while the Pi expands the filesystem and
applies your settings. After that, you should be able to connect to it via SSH:

```bash
ssh pi@photopainter.local
```

(Replace `pi` with your username and `photopainter` with whatever hostname you
chose.)

> **Tip:** If `photopainter.local` doesn't resolve, you may need to find the
> Pi's IP address from your router's admin page and connect with
> `ssh pi@192.168.x.x`.
