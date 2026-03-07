# Copilot Instructions

## Build & Test

```bash
cargo build                          # build default member (dashboard)
cargo build --workspace              # build all crates
cargo test --workspace               # run all tests
cargo test --package strava          # tests live in the strava crate
cargo test -p strava types::tests::test_zero_distance  # run a single test by name
```

Requires Rust edition 2024 (rust-version 1.93+). No CI pipeline exists yet.

## Architecture

Cargo workspace with two crates:

- **`strava`** (library) — Strava API client, data types, caching, and stats computation. This is where most logic and all tests live.
- **`dashboard`** (binary, default member) — Thin orchestrator that loads config, calls the `strava` crate, and prints results. Will eventually drive a Waveshare e-paper display on a Raspberry Pi Zero 2W.

### Data flow

`dashboard::main` → `strava::config::Config::load()` → `strava::client::Client` (authenticate, fetch) → `strava::stats::DashboardStats::compute()` → print/display

### Caching

`strava::cache::Cache` is a file-based JSON cache in `~/.cache/rpi-zero2w-strava-dash/`. Each API response is wrapped in a `CacheEntry` with a `fetched_at` timestamp and per-key TTL (default 3 hours, athlete data 7 days). The client checks cache before every API call.

### Configuration

TOML config at `~/.config/rpi-zero2w-strava-dash/config.toml` with Strava API credentials (`client_id`, `client_secret`, `refresh_token`). If the file is missing, a template is auto-created and the program exits with an error message.

## Conventions

- **Error handling**: `thiserror` enums per crate (`StravaError`, `DashError`). The dashboard crate defines a local `Result<T>` type alias.
- **HTTP client**: `reqwest::blocking` (not async). The README mentions async/Tokio but the current implementation is synchronous.
- **Serde**: All API response types derive `Serialize + Deserialize` with `#[serde(default)]` on optional fields. Field renames use `#[serde(rename = "...")]`.
- **Logging**: `log` macros everywhere, `env_logger` in the binary. Run with `RUST_LOG=info` (or `debug`) to see output.
- **Units**: Strava API returns meters/seconds. Conversion methods live on the types themselves (e.g., `distance_km()`, `avg_speed_kmh()`, `format_pace_per_km()`).
