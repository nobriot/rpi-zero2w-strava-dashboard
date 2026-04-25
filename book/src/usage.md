# Usage

The full list of flags is available from the binary itself:

```bash
strava-dashboard --help
```

This page only highlights the most common workflows. The config file lives
at `~/.config/rpi-zero2w-strava-dashboard/config.toml` by default; pass
`--config <path>` to use a different one.

## Default loop

```bash
strava-dashboard
```

Fetches the latest data, renders the dashboard, pushes it to the e-paper
display, then sleeps for the configured interval before refreshing again.
This is what the systemd service runs.

## Single cycle, save a PNG

Useful when iterating on configuration or running on a machine without an
e-paper display attached:

```bash
strava-dashboard --once --save-png /tmp/dashboard.png
```

Add `--scale 2` for a 1600x960 high-resolution preview.

## Kiosk mode

Kiosk mode renders the dashboard in a continuous loop, writes each frame to
a PNG, and skips all power management (no shutdown, no quiet hours, no
e-paper writes). Handy when the project runs on a regular screen instead of
the PhotoPainter, or for a live preview during development:

```bash
strava-dashboard --kiosk --save-png /tmp/dashboard.png
```

`--save-png` is required in this mode. Pair it with an image viewer that
auto-reloads (`feh --auto-reload`, `eog`, ...) to watch the dashboard
update.

## Force a fresh fetch

```bash
strava-dashboard --clear-cache
```

Wipes the per-athlete JSON cache so the next cycle re-fetches everything
from Strava. The cache lives under
`~/.cache/rpi-zero2w-strava-dashboard/<athlete_id>/`.

## Debug logging

```bash
RUST_LOG=debug strava-dashboard --once
```
