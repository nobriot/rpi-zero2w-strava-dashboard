# Running as a Service

To make the dashboard start automatically when the Pi boots (and restart if it
crashes), set it up as a **systemd service**.

## Install the service

If you used `just deploy`, the service file is already installed. Otherwise,
copy it manually:

```bash
# From your computer
scp install/strava-dashboard.service pi@photopainter.local:/tmp/

# On the Pi
ssh pi@photopainter.local
sudo mv /tmp/strava-dashboard.service /etc/systemd/system/
sudo systemctl daemon-reload
```

## Enable and start

```bash
# Enable = start on boot
sudo systemctl enable strava-dashboard

# Start it now
sudo systemctl start strava-dashboard
```

Or do both at once:

```bash
sudo systemctl enable --now strava-dashboard
```

## Check that it's running

```bash
sudo systemctl status strava-dashboard
```

You should see something like:

```
● strava-dashboard.service - Strava E-Paper Dashboard
     Loaded: loaded (/etc/systemd/system/strava-dashboard.service; enabled)
     Active: active (running) since ...
```

## Stop or restart

```bash
# Stop the service
sudo systemctl stop strava-dashboard

# Restart (e.g., after config changes)
sudo systemctl restart strava-dashboard

# Disable auto-start on boot
sudo systemctl disable strava-dashboard
```

## What the service does

The service file tells systemd to:

- Wait for network connectivity before starting
- Run `/usr/local/bin/strava-dashboard`
- Set log level to `info`
- Restart automatically if the program crashes (after a 30-second delay)
- Start on every boot

The dashboard then runs its own internal loop: fetch data, render, display,
sleep, repeat.
