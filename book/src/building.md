# Building & Deploying

At this point you should have:

- A Raspberry Pi ready with WiFi ([earlier steps](./sd-card.md))
- A `my-config.toml` with valid Strava credentials ([authorization](./strava-auth.md))
- Your display settings customized ([configuration](./configuration.md))

Now it's time to compile the software and deploy everything to the Pi.

## Install prerequisites

### 1. Install Rust

Open a terminal on your computer and run:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Follow the prompts (the defaults are fine). After installation, restart your
terminal or run:

```bash
source ~/.cargo/env
```

Verify it works:

```bash
rustc --version
# Should print something like: rustc 1.93.0 (...)
```

> **Note:** You need Rust 1.93 or newer.

### 2. Install Docker

`cross` (the cross-compilation tool) uses Docker behind the scenes.

- **Mac:** Install [Docker Desktop](https://www.docker.com/products/docker-desktop/)
- **Linux:** Install Docker via your package manager:
  ```bash
  # Ubuntu/Debian
  sudo apt install docker.io
  sudo usermod -aG docker $USER
  # Log out and back in for group change to take effect
  ```
- **Windows (WSL):** Install
  [Docker Desktop](https://www.docker.com/products/docker-desktop/) and enable
  WSL integration in settings.

### 3. Install [Just](https://just.systems/man/en/packages.html)

See their documentation: [https://just.systems/](https://just.systems/) and
[install page](https://just.systems/).

It could be for example (replacing DEST with the directory where just will be
installed):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://just.systems/install.sh | bash -s -- --to DEST
```

### Check everything

If you already cloned the repo during the [authorization step](./strava-auth.md),
run the automated setup that checks and installs all remaining tools:

```bash
just dev
```

This verifies your Rust installation, installs missing tools (`cross`, `mdbook`,
etc.), and does a test build.

## Build for the Raspberry Pi

Run the cross-compilation build:

```bash
just cross
```

Or without `just`:

```bash
cross build --release --target aarch64-unknown-linux-gnu
```

The first build downloads a Docker image and compiles all dependencies, which
can take 5--10 minutes. Subsequent builds are much faster.

When it finishes, the compiled program is at:

```
target/aarch64-unknown-linux-gnu/release/strava-dashboard
```

## Deploy to the Raspberry Pi

Make sure you can SSH into your Pi (see [Preparing the SD Card](./sd-card.md)).

### Using `just` (recommended)

```bash
# Deploy the binary and systemd service
just deploy pi@photopainter.local

# Deploy your config file
just deploy-config pi@photopainter.local my-config.toml
```

This compiles, copies the binary and systemd service to the Pi, deploys your
config file, and restarts the dashboard.

### Manually

```bash
# Copy the binary
scp target/aarch64-unknown-linux-gnu/release/strava-dashboard \
  pi@photopainter.local:/tmp/

# Copy the systemd service file
scp install/strava-dashboard.service \
  pi@photopainter.local:/tmp/

# SSH into the Pi and install
ssh pi@photopainter.local
sudo mv /tmp/strava-dashboard /usr/local/bin/
sudo mv /tmp/strava-dashboard.service /etc/systemd/system/
sudo systemctl daemon-reload

# Deploy the config (from your computer)
exit
scp my-config.toml \
  pi@photopainter.local:~/.config/rpi-zero2w-strava-dashboard/config.toml
```

> **Note:** You may need to create the config directory on the Pi first:
> `ssh pi@photopainter.local 'mkdir -p ~/.config/rpi-zero2w-strava-dashboard'`

## Verify it works

SSH into the Pi and run a single cycle:

```bash
ssh pi@photopainter.local
strava-dashboard --once
```

You should see your stats appear on the e-paper display after about 15 seconds.

## Next step

The dashboard is now installed and working. Continue to
[Running as a Service](./service.md) to make it start automatically on boot.
