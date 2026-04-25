# Building

For most cases, prefer `cargo install --git ...` (see the
[Introduction](./introduction.md)). This page covers building from a
clone -- useful for development, or when cross-compiling to the Raspberry
Pi from a more powerful machine.

## Prerequisites

### Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
rustc --version    # 1.93+ required
```

### Docker (for cross-compilation only)

`cross` runs the toolchain inside a Docker container. Install Docker via
your package manager or [Docker Desktop](https://www.docker.com/products/docker-desktop/),
and add yourself to the `docker` group on Linux:

```bash
sudo usermod -aG docker $USER
# log out and back in
```

### Just (optional)

The repo ships a [`Justfile`](https://just.systems/) with shortcuts for
common tasks. Install via the
[upstream instructions](https://just.systems/man/en/packages.html).

## Native build

```bash
git clone https://github.com/nobriot/rpi-zero2w-strava-dashboard
cd rpi-zero2w-strava-dashboard
cargo build --release
```

The binary lands at `target/release/strava-dashboard`. Run it directly or
copy it onto your `PATH`.

## Cross-compile for the Raspberry Pi

```bash
cargo install cross   # once
just cross            # or: cross build --release --target aarch64-unknown-linux-gnu
```

Output: `target/aarch64-unknown-linux-gnu/release/strava-dashboard`.

The first build downloads a Docker image and compiles every dependency
(5--10 min). Later builds reuse the cache.

## Deploy to the Pi

The repo's `Justfile` has shortcuts:

```bash
just deploy pi@photopainter.local                 # binary + service unit
just deploy-config pi@photopainter.local config.toml
```

Or do it manually:

```bash
scp target/aarch64-unknown-linux-gnu/release/strava-dashboard pi@photopainter.local:/tmp/
scp dist/strava-dashboard.service pi@photopainter.local:/tmp/

ssh pi@photopainter.local
sudo mv /tmp/strava-dashboard /usr/local/bin/
sudo mv /tmp/strava-dashboard.service /etc/systemd/system/
sudo systemctl daemon-reload

# from your machine:
ssh pi@photopainter.local 'mkdir -p ~/.config/rpi-zero2w-strava-dashboard'
scp config.toml pi@photopainter.local:~/.config/rpi-zero2w-strava-dashboard/
```

Then enable the service -- see [Running as a Service](./service.md).

## Smoke-test on the Pi

```bash
ssh pi@photopainter.local
strava-dashboard --once
```

The e-paper display should refresh in about 15 seconds.
