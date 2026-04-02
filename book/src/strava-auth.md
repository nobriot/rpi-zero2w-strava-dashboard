# Strava Authorization

Now that you have a Strava API app with a Client ID and Client Secret, you need
to **authorize** the dashboard to access your data. This is done on your
development machine (the computer where you'll build the software), and it
produces a configuration file with all the credentials the dashboard needs.

## Step 1: Get the source code

If you haven't already, clone the project repository:

```bash
git clone https://github.com/nobriot/rpi-zero2w-strava-dashboard.git
cd rpi-zero2w-strava-dashboard
```

## Step 2: Create an initial config file

Copy the example configuration and fill in your Strava credentials:

```bash
cp dist/config.example.toml my-config.toml
```

Open `my-config.toml` in a text editor and enter your Client ID and Client
Secret from the [previous step](./strava-app.md):

```toml
[strava]
client_id = "123456"
client_secret = "abcdef1234567890abcdef1234567890abcdef12"
refresh_token = ""
```

Leave `refresh_token` empty --- the next step will fill it in automatically.

## Step 3: Run the authorization flow

Run the dashboard with the `--auth` flag, pointing it at your config file:

```bash
cargo run -- --auth --config my-config.toml
```

> **Note:** The first time you run this, Rust will download and compile all
> dependencies. This can take a few minutes. Subsequent runs are fast.

This will:

1. Start a temporary web server on your machine
2. Print a URL to your terminal --- **open it in your browser**
3. Show the Strava authorization page where you click **"Authorize"**
4. Automatically receive the authorization code and exchange it for tokens
5. Save the **refresh token** into your config file

After this, your `my-config.toml` will have a valid `refresh_token` filled in.

## Step 4: Verify it works

Test that the dashboard can fetch your data and render a preview:

```bash
cargo run -- --once --save-png test.png --config my-config.toml
```

This fetches your activities from Strava and saves a rendered dashboard image
as `test.png`. Open it to check that your stats look correct.

## What happens behind the scenes

When you authorize:

1. You grant the dashboard **read-only** access to your activities and profile
2. Strava gives back an **authorization code** (one-time use)
3. The dashboard exchanges this code for an **access token** (expires in 6
   hours) and a **refresh token** (long-lived)
4. The refresh token is saved in your config file
5. On each cycle, the dashboard uses the refresh token to get a fresh access
   token --- no further manual authorization is needed

If the refresh token ever becomes invalid (e.g., you revoke the app on
Strava's website), the dashboard will detect the 401 error and automatically
prompt for re-authorization.

## Next step

You now have a working config file with valid Strava credentials. Continue to
[Configuration](./configuration.md) to customize your goals and display
settings before deploying to the Pi.
