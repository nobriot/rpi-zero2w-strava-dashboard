# Command-Line Options

The `strava-dashboard` binary accepts the following command-line options.

## Reference

Get the available options by running 

```bash
strava-dashboard --help
```

## Common usage examples

### Initial Strava authorization

Run the auth flow, get a strava API token.

```bash
strava-dashboard --auth
```

Opens the Strava authorization flow. Do this once after installing.

### Test without the e-paper display

```bash
strava-dashboard --once --save-png ~/dashboard-preview.png
```

Fetches your data, renders the dashboard, and saves it as a PNG image you can
view on any computer. The `--once` flag makes it exit after one cycle instead
of looping.

### High-resolution preview

```bash
strava-dashboard --once --save-png ~/dashboard-hires.png --scale 2
```

Saves at 2x resolution (1600 x 960 pixels) for a sharper image.
Won't have any effect on the display, useful for local visualization.

### Use a different config file

```bash
strava-dashboard --config /path/to/my-config.toml --once
```

### Clear stale cache

```bash
strava-dashboard --clear-cache
```

Deletes all cached Strava data. The next run will fetch everything fresh.

### Demo mode

```bash
strava-dashboard --once --show-all-sports --save-png demo.png
```

Shows all three sport sections (run, ride, swim) even if you don't have
activities in all of them. Useful for seeing what the full dashboard looks
like.

### Debug logging

```bash
RUST_LOG=debug strava-dashboard --once
```

Prints detailed debug information. Useful for diagnosing issues.
