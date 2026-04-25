# Troubleshooting

## The display stays blank

Check that the service is running:

```bash
sudo systemctl status strava-dashboard
sudo journalctl -u strava-dashboard -n 50
```

Check that SPI is enabled:

```bash
ls /dev/spidev0.0
```

If the device file is missing, ensure `dtparam=spi=on` is in
`/boot/firmware/config.txt` (or `/boot/config.txt`) and reboot.

Check that I2C is enabled (only needed for the INA219 battery monitor):

```bash
ls /dev/i2c-1
```

If missing, add `dtparam=i2c_arm=on` to `config.txt` and reboot.

## "Unauthorized" / 401 errors in the logs

The dashboard normally refreshes the access token on its own. If it keeps
failing:

1. Re-run the auth flow:

   ```bash
   strava-dashboard --auth
   ```

2. Confirm `client_id` and `client_secret` in `config.toml` match the
   values shown at <https://www.strava.com/settings/api>.
3. Check that the API app is still active there (deleting/recreating it
   invalidates old refresh tokens).

## Can't SSH into the Pi

```bash
ping photopainter.local
```

If that fails, find the Pi's IP from your router admin page or via
`arp -a`, and SSH using the IP directly. If you flashed your own SD card,
make sure SSH was enabled via the Raspberry Pi Imager settings (or by
creating an empty `ssh` file on the boot partition).

## WiFi not connecting

```bash
nmcli connection show
nmcli device wifi list
ls -la /etc/NetworkManager/system-connections/
```

NetworkManager profiles must be owned by `root` with `600` permissions:

```bash
sudo chmod 600 /etc/NetworkManager/system-connections/YourWiFi.nmconnection
sudo nmcli connection reload
```

## Dashboard shows stale data

This is expected behavior when WiFi is temporarily unavailable -- the
dashboard falls back to its on-disk cache and refreshes once connectivity
returns. To force a full refetch:

```bash
strava-dashboard --clear-cache
sudo systemctl restart strava-dashboard
```

## Garbled or wrong colors on the display

The 6-color e-paper takes about 12 seconds to refresh. Losing power
mid-refresh can leave the panel in a partial state -- let the next cycle
complete and it should clear. If colors stay wrong, stop the service and
power-cycle the Pi to force a full clear on the next start.

## Build errors

**Rust too old** -- update with `rustup update`. Minimum is 1.93.

**Docker not running** (cross-compile fails) -- start the daemon
(`sudo systemctl start docker` on Linux, launch Docker Desktop on Mac/
Windows) and verify with `docker run hello-world`.

**Permission denied on Docker** -- `sudo usermod -aG docker $USER`, then
log out and back in.

## Where to get help

If something isn't covered here, check the
[GitHub issues](https://github.com/nobriot/rpi-zero2w-strava-dashboard/issues)
for known problems or open a new one with logs and a description of your
setup.
