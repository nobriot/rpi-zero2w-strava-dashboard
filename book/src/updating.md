# Updating the Dashboard

When a new version of the dashboard is available, you can update it by
rebuilding and redeploying.

## Update the source code

On your computer, pull the latest changes:

```bash
cd rpi-zero2w-strava-dash
git pull
```

## Rebuild and deploy

### Using `just`

```bash
just deploy pi@photopainter.local
```

This cross-compiles the new version, copies it to the Pi, and restarts the
service automatically.

### Manually

```bash
# Rebuild
cross build --release --target aarch64-unknown-linux-gnu

# Copy to Pi
scp target/aarch64-unknown-linux-gnu/release/strava-dashboard \
  pi@photopainter.local:/tmp/

# On the Pi: replace binary and restart
ssh pi@photopainter.local
sudo systemctl stop strava-dashboard
sudo mv /tmp/strava-dashboard /usr/local/bin/
sudo systemctl start strava-dashboard
```

## Verify the update

Check that the new version is running:

```bash
ssh pi@photopainter.local
sudo systemctl status strava-dashboard
sudo journalctl -u strava-dashboard -n 20
```

Your configuration file is not affected by updates --- it stays in place and
keeps working.
