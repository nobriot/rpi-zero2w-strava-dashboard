# Strava Authorization

Once you have a Strava API app with a Client ID and Client Secret (see
[Creating a Strava API App](./strava-app.md)), the next step is to authorize
the dashboard and write a refresh token into the config file. This is a
one-time setup.

## 1. Seed a config file

Copy the example config and paste in the credentials from the previous
step. Leave `refresh_token` empty -- the auth flow fills it in.

```bash
mkdir -p ~/.config/rpi-zero2w-strava-dashboard
curl -fsSL https://raw.githubusercontent.com/nobriot/rpi-zero2w-strava-dashboard/main/dist/config.example.toml \
    -o ~/.config/rpi-zero2w-strava-dashboard/config.toml
$EDITOR ~/.config/rpi-zero2w-strava-dashboard/config.toml
```

```toml
[strava]
client_id = "123456"
client_secret = "abcdef..."
refresh_token = ""
```

## 2. Run the auth flow

```bash
strava-dashboard --auth
```

This starts a local web server, prints a URL, opens the Strava authorization
page in your browser, and -- once you click **Authorize** -- writes the
refresh token back into the config file. Read-only scope is requested; the
dashboard cannot modify or delete anything on your account.

## 3. Verify

```bash
strava-dashboard --once --save-png /tmp/dashboard.png
```

Open the PNG to confirm your stats render correctly.

If the refresh token is ever revoked, the dashboard returns a 401 and you
can re-run `strava-dashboard --auth` to issue a new one.
