# Creating a Strava API App

Before the dashboard can access your Strava data, you need to create a
"Strava API Application". This is free and takes about 2 minutes.

## Step 1: Go to the Strava API page

Open your browser and go to **<https://www.strava.com/settings/api>**.

Log in with your Strava account if prompted. If you've never created an API
app before, you'll see a **"Create an App"** button. If you already have
one, you'll see **"My API Application"**.

## Step 2: Fill in the form

| Field                              | What to enter                       |
|------------------------------------|-------------------------------------|
| **Application Name**               | Anything you like, e.g. `RPi Dash`  |
| **Category**                       | Pick any (e.g. "Visualizer")        |
| **Club**                           | Leave empty                         |
| **Website**                        | `http://localhost`                  |
| **Authorization Callback Domain**  | `localhost`                         |

Click **"Create"**.

## Step 3: Copy your credentials

After creating the app, you'll see two important values:

- **Client ID** -- a number (e.g. `123456`)
- **Client Secret** -- a long string of letters and numbers

Keep them handy -- the next step writes them into your config file.

> **Important:** keep your Client Secret private. Anyone with it can read
> your Strava data. Don't share it publicly or commit it to version
> control.

## What these credentials do

- The **Client ID** identifies your app to Strava.
- The **Client Secret** proves your app is authorized.
- Together with a **Refresh Token** -- obtained in the
  [next chapter](./strava-auth.md) -- they let the dashboard fetch your
  activities, totals, and profile info on every cycle.

The dashboard only requests **read-only** access to your activities. It
cannot modify, delete, or create anything on your Strava account.

## Next step

Continue to [Strava Authorization](./strava-auth.md) to run the auth flow
and produce a refresh token.
