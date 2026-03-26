# List available recipes
default:
    @just --list

# Run all workspace tests (unit + snapshot)
test:
    RUST_MIN_STACK=16777216 cargo test --workspace

# Run only unit tests (strava crate)
test-unit:
    cargo test --package strava

# Run snapshot tests — render all configs and compare against references
snapshot:
    RUST_MIN_STACK=16777216 cargo test -p display --test snapshots -- --nocapture

# Update reference snapshots after intentional visual changes
snapshot-update:
    RUST_MIN_STACK=16777216 UPDATE_SNAPSHOTS=1 cargo test -p display --test snapshots -- --nocapture

# Render a single preview PNG from live/cached data (default config)
preview:
    cargo run -- --once --save-png tmp/test.png --scale 1

# Render a preview for a specific test config
preview-config config:
    cargo run -- --once --config tests/{{config}}.toml --save-png tmp/{{config}}.png --scale 1

# Lint: clippy + format
lint:
    cargo clippy --all-targets --all-features -- -D warnings
    cargo +nightly fmt --all

# Build for RPi Zero 2W
cross:
    cross build --release --target aarch64-unknown-linux-gnu

# Cross-compile and deploy to RPi (e.g. just deploy pi@192.168.0.45)
deploy host:
    cross build --release --target aarch64-unknown-linux-gnu
    scp target/aarch64-unknown-linux-gnu/release/strava-dashboard {{host}}:/tmp/strava-dashboard
    scp install/strava-dashboard.service {{host}}:/tmp/strava-dashboard.service
    ssh {{host}} 'sudo mv /tmp/strava-dashboard /usr/local/bin/strava-dashboard && sudo mv /tmp/strava-dashboard.service /etc/systemd/system/strava-dashboard.service && sudo systemctl daemon-reload && sudo systemctl restart strava-dashboard'
    @echo "Deployed and restarted strava-dashboard on {{host}}"
