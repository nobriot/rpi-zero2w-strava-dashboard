# Configuration

All settings live in a single file:

```
~/.config/rpi-zero2w-strava-dashboard/config.toml
```

Here's a complete example with explanations for each option.

## Strava credentials (required)

```toml
[strava]
client_id = "123456"
client_secret = "abcdef1234567890..."
refresh_token = "xyz789..."
```

These are set during the [Strava setup](./strava-app.md) and
[authorization](./strava-auth.md) steps. You generally don't need to edit these
after the initial setup.

## Display settings (all optional)

```toml
[display]
```

### Refresh interval

```toml
sleep_interval_secs = 10800
```

How often the dashboard refreshes, in seconds. Default is **10800** (3 hours).

| Value | Interval |
|-------|----------|
| 3600 | 1 hour |
| 7200 | 2 hours |
| 10800 | 3 hours (default) |
| 21600 | 6 hours |
| 43200 | 12 hours |

> **Tip:** The Strava API has rate limits (100 requests per 15 minutes, 1000
> per day). A 3-hour interval uses about 8 requests per day, well within
> limits. Don't set this below 1 hour.

### Quiet hours

```toml
quiet_start_hour = 20
quiet_end_hour = 6
```

The dashboard won't refresh between these hours (local time). Default is
**20:00 to 06:00** (8 PM to 6 AM). This saves power and avoids the display
refreshing while you're asleep.

Set both to the same value to disable quiet hours.

### Show/hide sections

```toml
show_totals = true
show_longest_fastest = true
```

- **show_totals** --- Show the TOTALS row (activity count, total distance,
  time, elevation, kudos). Default: `true`.
- **show_longest_fastest** --- Show the LONGEST / FASTEST section with per-sport
  records and race bests. Default: `true`.

Hiding sections gives more space to the remaining elements.

### Route line thickness

```toml
polyline_thickness = 4
```

The thickness (in pixels) of the route line drawn on the map for your last
activity. Default: **4**. Increase for a bolder line, decrease for a thinner
one.

### Low-power mode

```toml
shutdown_after_cycle = false
```

When set to `true`, the Pi disables WiFi and HDMI between refresh cycles to
extend battery life. WiFi is re-enabled 10 seconds before the next cycle.
Default: `false`.

This roughly halves power consumption during sleep (from ~120 mA to ~50-60 mA).

### Sport goals

```toml
[[display.goals]]
sport = "ride"
km = 5000.0

[[display.goals]]
sport = "run"
km = 500.0

[[display.goals]]
sport = "swim"
km = 30.0
```

Define 1 to 3 yearly distance goals. These appear as progress bars at the top
of the dashboard. Available sports: `"run"`, `"ride"`, `"swim"`.

**Order matters:**
- The **first** sport gets a full-width progress bar on the top row
- With 2 sports, each gets a full-width bar
- With 3 sports, the second and third share the second row as half-width bars

Adjust the `km` values to match your personal goals for the year.

## Full example

```toml
[strava]
client_id = "123456"
client_secret = "abcdef1234567890abcdef1234567890abcdef12"
refresh_token = "fedcba0987654321fedcba0987654321fedcba09"

[display]
sleep_interval_secs = 10800
quiet_start_hour = 22
quiet_end_hour = 7
show_totals = false
show_longest_fastest = true
shutdown_after_cycle = false
polyline_thickness = 4

[[display.goals]]
sport = "ride"
km = 5000.0

[[display.goals]]
sport = "run"
km = 500.0

[[display.goals]]
sport = "swim"
km = 30.0
```

## Applying changes

After editing the config file on the Pi, restart the dashboard service:

```bash
sudo systemctl restart strava-dashboard
```

Or, if you edited it on your computer, re-deploy:

```bash
just deploy-config pi@photopainter.local my-config.toml
```

The deploy command copies the file and the service will pick up changes on its
next cycle.
