# Copilot Instructions

## Git workflow

Do **not** commit changes. Only modify code and let the user review and commit manually.

## Build & Test

```bash
cargo build                          # build default member (dashboard)
cargo build --workspace              # build all crates
cargo test --workspace               # run all tests
cargo test --package strava          # tests live in the strava crate
cargo test -p strava types::tests::test_zero_distance  # run a single test by name
```

Requires Rust edition 2024 (rust-version 1.93+). Cross-compiled for `aarch64-unknown-linux-gnu` (RPi Zero 2W) via `Cross.toml`. No CI pipeline exists yet.

## Architecture

Cargo workspace with six crates — three libraries and three binaries:

### Libraries

- **`strava`** (lib) — Strava API client, OAuth, data types, file-based caching, and stats aggregation. Most logic and all tests live here.
- **`display`** (lib) — Waveshare 7.3" ACeP e-paper driver (SPI/GPIO via `rppal`), INA219 battery monitor (I2C), and dashboard image renderer (`image` + `imageproc`). Depends on `strava` for `DashboardStats`.
- **`protocol`** (lib) — Shared nd-JSON wire protocol (request/response enums) for USB serial communication between `usbd` and `console`. No workspace dependencies.

### Binaries

- **`dashboard`** (bin, default member) — Main loop on the RPi: fetch Strava stats → render 800×480 image → send to e-paper display. Handles quiet hours, auto-refresh, battery monitoring, and offline fallback. Depends on `strava` and `display`.
- **`usbd`** (bin, `strava-usbd`) — USB serial daemon on the RPi. Listens on `/dev/ttyGS0`, handles JSON-lines requests for WiFi management (`nmcli`), config get/push, system status, and dashboard refresh. Depends on `protocol` and `strava`.
- **`console`** (bin, `strava-console`) — Interactive CLI on the dev machine. Auto-detects USB serial, runs a guided setup wizard (WiFi + Strava OAuth), then drops into a REPL for device management. Depends on `protocol`, `strava`, and `serialport`.

### Dependency graph

```
strava (lib)
├──► display (lib)
│    └──► dashboard (bin)       [RPi: main loop]
├──► usbd (bin)                 [RPi: USB daemon]
└──► console (bin)              [Dev: setup CLI]

protocol (lib)
├──► usbd (bin)
└──► console (bin)
```

### Data flow

**Dashboard cycle (RPi):**
`dashboard::main` → `Config::load()` → `Client` (authenticate, fetch, cache) → `DashboardStats::compute()` → `render_dashboard()` → `Epd7in3e::display_image()` → sleep → repeat

**USB setup (dev ↔ RPi):**
`strava-console` ↔ JSON lines over USB serial ↔ `strava-usbd` → WiFi (`nmcli`), config.toml, dashboard restart

### Caching

`strava::cache::Cache` is a file-based JSON cache in `~/.cache/rpi-zero2w-strava-dash/`. Each API response is wrapped in a `CacheEntry` with a `fetched_at` timestamp and per-key TTL (default 3 hours, athlete data 7 days). The client checks cache before every API call.

### Configuration

TOML config at `~/.config/rpi-zero2w-strava-dash/config.toml`:

```toml
[strava]
client_id = "..."
client_secret = "..."
refresh_token = "..."

[display]
sleep_interval_secs = 10800   # 3 hours
quiet_start_hour = 20         # no refresh 20:00–08:00
quiet_end_hour = 8
run_goal_km = 2000.0
ride_goal_km = 5000.0
swim_goal_km = 200.0
```

If the file is missing, a template is auto-created and the program exits with an error message.

### Network resilience

- **401 Unauthorized**: Client auto-refreshes OAuth token (retry once), persists new `refresh_token` to config.
- **Network unavailable**: `NetworkUnavailable` error for DNS/connect/timeout failures; dashboard shows offline screen with battery % and retries next cycle.

## Conventions

- **Error handling**: `thiserror` enums per crate (`StravaError`, `DisplayError`, `DashError`). Each binary crate defines a local `Result<T>` type alias.
- **HTTP client**: `reqwest::blocking` (synchronous, not async).
- **Serde**: All API response types derive `Serialize + Deserialize` with `#[serde(default)]` on optional fields. Field renames use `#[serde(rename = "...")]`.
- **Logging**: `log` macros everywhere, `env_logger` in binaries. Run with `RUST_LOG=info` (or `debug`) to see output.
- **Units**: Strava API returns meters/seconds. Conversion methods live on the types themselves (e.g., `distance_km()`, `avg_speed_kmh()`, `format_pace_per_km()`).
