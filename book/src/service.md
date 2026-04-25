# Running as a Service

To start the dashboard automatically on boot and restart it on crash, install
it as a systemd service.

## Install the unit file

A ready-to-use unit lives at `dist/strava-dashboard.service` in the repo.
Adjust the `ExecStart` path and `--config` path if your binary or config
file are elsewhere, then install it:

```bash
sudo cp dist/strava-dashboard.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now strava-dashboard
```

The default unit assumes:

- Binary at `/usr/local/bin/strava-dashboard`
  (move or symlink your `~/.cargo/bin/strava-dashboard` there, or edit the
  `ExecStart` path)
- Config at `/home/pi/.config/rpi-zero2w-strava-dashboard/config.toml`
- Logs appended to `/var/log/strava-dashboard.log`

## Operate the service

```bash
sudo systemctl status strava-dashboard      # is it running?
sudo systemctl restart strava-dashboard     # apply config changes
sudo systemctl stop strava-dashboard
sudo systemctl disable strava-dashboard     # don't start on next boot
journalctl -u strava-dashboard -f           # follow logs live
```

## What it does

The unit waits for `network-online.target`, runs `strava-dashboard` with the
configured paths and `RUST_LOG=info`, and restarts the binary 30 seconds
after any crash. The dashboard handles its own fetch -> render -> display ->
sleep loop internally; systemd just keeps the process alive.
