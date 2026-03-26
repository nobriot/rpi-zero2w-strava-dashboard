# List available recipes
default:
    @just --list

# Set up the development environment
dev:
    #!/usr/bin/env bash
    set -euo pipefail
    ok="\033[0;32m✓\033[0m"
    err="\033[0;31m✗\033[0m"
    info="\033[0;34m→\033[0m"
    missing=()

    # --- rustup & cargo ---
    if ! command -v rustup &>/dev/null; then
        echo -e "$err rustup not found — install from https://rustup.rs"
        exit 1
    fi
    echo -e "$ok rustup $(rustup --version 2>/dev/null | head -1 | awk '{print $2}')"

    if ! command -v cargo &>/dev/null; then
        echo -e "$err cargo not found"
        exit 1
    fi
    echo -e "$ok cargo $(cargo --version | awk '{print $2}')"

    # --- stable toolchain ---
    if ! rustup toolchain list | grep -q '^stable'; then
        echo -e "$info Installing stable toolchain..."
        rustup toolchain install stable
    fi
    echo -e "$ok stable toolchain"

    # --- nightly toolchain (needed for cargo +nightly fmt) ---
    if ! rustup toolchain list | grep -q '^nightly'; then
        echo -e "$info Installing nightly toolchain..."
        rustup toolchain install nightly
    fi
    echo -e "$ok nightly toolchain"

    # --- components: clippy, rustfmt, rust-analyzer, rust-src ---
    for comp in clippy rustfmt rust-analyzer rust-src; do
        if ! rustup component list --installed | grep -q "^${comp}"; then
            echo -e "$info Installing component $comp..."
            rustup component add "$comp"
        fi
        echo -e "$ok $comp"
    done

    # --- nightly rustfmt (for cargo +nightly fmt) ---
    if ! rustup component list --installed --toolchain nightly | grep -q "^rustfmt"; then
        echo -e "$info Installing nightly rustfmt..."
        rustup component add rustfmt --toolchain nightly
    fi
    echo -e "$ok nightly rustfmt"

    # --- cross ---
    if ! command -v cross &>/dev/null; then
        echo -e "$info Installing cross..."
        cargo install cross
    fi
    echo -e "$ok cross $(cross --version 2>/dev/null | head -1 | awk '{print $2}')"

    # --- aarch64 target (for cross-compilation) ---
    if ! rustup target list --installed | grep -q 'aarch64-unknown-linux-gnu'; then
        echo -e "$info Adding aarch64-unknown-linux-gnu target..."
        rustup target add aarch64-unknown-linux-gnu
    fi
    echo -e "$ok aarch64-unknown-linux-gnu target"

    # --- optional tools ---
    for tool in scp ssh; do
        if command -v "$tool" &>/dev/null; then
            echo -e "$ok $tool"
        else
            missing+=("$tool")
        fi
    done

    # --- verify build ---
    echo ""
    echo -e "$info Building workspace..."
    cargo build --workspace
    echo -e "$ok workspace builds"

    if [ ${#missing[@]} -gt 0 ]; then
        echo ""
        echo -e "$err Optional tools missing: ${missing[*]} (needed for deploy)"
    fi
    echo ""
    echo "Dev environment ready. Run 'just' to see available recipes."

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
