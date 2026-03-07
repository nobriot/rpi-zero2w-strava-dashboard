# Strava API Setup

## Create a Strava API Application

1. Go to https://www.strava.com/settings/api
2. Click **"Create an App"** (or "My API Application" if you have one)
3. Fill in the form:
   - **Application Name**: "RPi Strava Dash" (or your choice)
   - **Category**: Choose appropriate category
   - **Club**: Leave empty
   - **Website**: Your website or `http://localhost`
   - **Authorization Callback Domain**: `localhost`
4. Click **"Create"**

## Get Your Credentials

After creating, you'll see:
- **Client ID** (numerical)
- **Client Secret** (long string)

Copy these into your `config.toml`.

## Get Refresh Token

You need to authorize your app once to get a refresh token.

### Method 1: Built-in Authorization (Recommended)

Run the dashboard with the `--auth` flag:

```bash
dashboard --auth
```

This will:
1. Start a temporary local server on a random port
2. Print the Strava authorization URL to the terminal
3. Wait for you to open the URL and click **"Authorize"**
4. Automatically exchange the authorization code for tokens
5. Save the `refresh_token` to your `config.toml`

> **Tip:** If running on a headless device (e.g. RPi), open the printed URL on another
> machine and set up SSH port forwarding so the redirect reaches the Pi
> (e.g. `ssh -L <port>:localhost:<port> pi@<host>`).

The dashboard also **automatically re-authorizes** if it receives a 401 Unauthorized
response during normal operation — no manual intervention needed.

### Method 2: Manual Authorization

1. Visit this URL in your browser (replace `YOUR_CLIENT_ID`):


```
https://www.strava.com/oauth/authorize?client_id=YOUR_CLIENT_ID&response_type=code&redirect_uri=http://localhost&approval_prompt=force&scope=activity:read_all
```

2. Click **"Authorize"**

3. You'll be redirected to `http://localhost/?code=AUTHORIZATION_CODE`

4. Copy the `code` from the URL

5. Exchange it for tokens using `curl`:

```bash
curl -X POST https://www.strava.com/oauth/token \
  -d client_id=YOUR_CLIENT_ID \
  -d client_secret=YOUR_CLIENT_SECRET \
  -d code=AUTHORIZATION_CODE \
  -d grant_type=authorization_code
```

6. The response contains your `refresh_token` - copy this to `config.toml`

## Permissions

The app requests `activity:read_all` scope to:
- Read your activities
- Access activity details (distance, time, routes)

---

**Next:** Run the dashboard!
