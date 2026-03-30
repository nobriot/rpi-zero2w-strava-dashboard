# Checking Logs & Status

## Viewing logs

The dashboard logs to the system journal. To see what it's doing:

```bash
# Show recent logs
sudo journalctl -u strava-dashboard

# Follow logs in real time (Ctrl+C to stop)
sudo journalctl -u strava-dashboard -f

# Show only the last 50 lines
sudo journalctl -u strava-dashboard -n 50

# Show logs since today
sudo journalctl -u strava-dashboard --since today
```

### What the logs look like

A normal cycle looks something like:

```
[INFO] Starting dashboard cycle
[INFO] Authenticating with Strava
[INFO] Fetching athlete profile
[INFO] Fetching activities for 2026
[INFO] Rendering dashboard (800x480)
[INFO] Displaying on e-paper (12s refresh)
[INFO] Sleeping for 10800 seconds
```

### Increasing log detail

For more detailed logs (useful for debugging), edit the service to change the
log level:

```bash
sudo systemctl edit strava-dashboard
```

Add:

```ini
[Service]
Environment=RUST_LOG=debug
```

Then restart:

```bash
sudo systemctl restart strava-dashboard
```

To reset to normal logging, remove the override:

```bash
sudo systemctl revert strava-dashboard
sudo systemctl restart strava-dashboard
```

## Checking service status

```bash
sudo systemctl status strava-dashboard
```

This shows whether the service is running, when it started, and the last few
log lines.

## Checking battery level

The battery level is shown on the dashboard display itself (top-right corner).
You can also check it in the logs --- look for lines mentioning battery voltage
or percentage.

## Checking WiFi

```bash
# Connection status
nmcli connection show --active

# Signal strength
iwconfig wlan0

# Test internet connectivity
ping -c 3 strava.com
```
