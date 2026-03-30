# Troubleshooting

## The display stays blank

**Check that the service is running:**

```bash
sudo systemctl status strava-dashboard
```

If it shows `inactive` or `failed`, check the logs:

```bash
sudo journalctl -u strava-dashboard -n 50
```

**Check that SPI is enabled:**

```bash
ls /dev/spidev0.0
```

If the file doesn't exist, SPI is not enabled. Edit `/boot/firmware/config.txt`
(or `/boot/config.txt`), make sure `dtparam=spi=on` is present, and reboot:

```bash
sudo reboot
```

**Check that I2C is enabled** (for the battery monitor):

```bash
ls /dev/i2c-1
```

If missing, add `dtparam=i2c_arm=on` to config.txt and reboot.

## "Unauthorized" or authentication errors

If the logs show 401 errors or authentication failures:

1. The dashboard will normally handle this automatically by re-running the
   auth flow.
2. If it keeps failing, re-run the authorization manually:

   ```bash
   strava-dashboard --auth
   ```

3. Make sure your Client ID and Client Secret in `config.toml` are correct.

4. Check that your Strava API app is still active at
   <https://www.strava.com/settings/api>.

## Can't connect to the Pi via SSH

**Check that the Pi is on your network:**

```bash
ping photopainter.local
```

If that doesn't work, try finding it by IP:

- Check your router's admin page for connected devices
- Try `arp -a` on your computer to scan local devices

**Check that SSH is enabled:**

If you're using the pre-flashed SD card, SSH should be enabled by default. If
you flashed your own, make sure you enabled SSH in the Raspberry Pi Imager
settings, or create an empty file called `ssh` on the boot partition of the
SD card.

## WiFi not connecting

**Check NetworkManager status:**

```bash
nmcli connection show
nmcli device wifi list
```

**Verify the connection file permissions:**

```bash
ls -la /etc/NetworkManager/system-connections/
```

Files must be owned by root with `600` permissions. Fix with:

```bash
sudo chmod 600 /etc/NetworkManager/system-connections/YourWiFi.nmconnection
sudo nmcli connection reload
```

## Dashboard shows stale data

This is normal when WiFi is temporarily unavailable. The dashboard uses cached
data and will refresh when connectivity returns.

To force a refresh of all cached data:

```bash
strava-dashboard --clear-cache
sudo systemctl restart strava-dashboard
```

## The display looks garbled or has wrong colors

The 6-color e-paper display takes about 12 seconds to refresh. If the Pi loses
power during a refresh, the display may look garbled. Simply let it complete
a full refresh cycle.

If colors consistently look wrong, the display hardware may need a full clear.
Stop the service and power-cycle the Pi.

## Build errors on your computer

**"Rust version too old":**

```bash
rustup update
rustc --version  # Should be 1.93+
```

**Docker not running** (cross-compilation fails):

```bash
# Start Docker
sudo systemctl start docker   # Linux
# Or open Docker Desktop       # Mac/Windows

# Verify Docker works
docker run hello-world
```

**Permission denied on Docker:**

```bash
sudo usermod -aG docker $USER
# Log out and back in
```

## Where to get help

If your issue isn't covered here, check the
[GitHub issues](https://github.com/nwoltman/rpi-zero2w-strava-dash/issues)
for known problems and solutions, or open a new issue describing your problem.
