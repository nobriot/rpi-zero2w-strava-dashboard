use crate::config::StravaConfig;
use crate::errors::StravaError;
use crate::types::TokenResponse;
use reqwest::blocking::Client as ReqwestClient;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;

const STRAVA_AUTHORIZE_URL: &str = "https://www.strava.com/oauth/authorize";
const STRAVA_TOKEN_URL: &str = "https://www.strava.com/oauth/token";

const SUCCESS_HTML: &str = r#"<!DOCTYPE html>
<html><head><title>Authorization Successful</title></head>
<body style="font-family:sans-serif;text-align:center;padding:2em">
<h1>&#10003; Authorization Successful</h1>
<p>You can close this tab and return to the terminal.</p>
</body></html>"#;

const ERROR_HTML: &str = r#"<!DOCTYPE html>
<html><head><title>Authorization Failed</title></head>
<body style="font-family:sans-serif;text-align:center;padding:2em">
<h1>&#10007; Authorization Failed</h1>
<p>The authorization was denied or an error occurred. Please try again.</p>
</body></html>"#;

/// Run the full OAuth Authorization Code flow:
/// 1. Start a local HTTP server on a random port
/// 2. Open the Strava authorization page in the user's browser
/// 3. Wait for the redirect callback with the authorization code
/// 4. Exchange the code for tokens
pub fn run_auth_flow(config: &StravaConfig) -> Result<TokenResponse, StravaError> {
  let listener = TcpListener::bind("127.0.0.1:0")
    .map_err(|e| StravaError::OAuthError(format!("Failed to start local server: {e}")))?;

  let port =
    listener.local_addr()
            .map_err(|e| StravaError::OAuthError(format!("Failed to get local address: {e}")))?
            .port();

  let redirect_uri = format!("http://localhost:{port}");
  let authorize_url = format!("{STRAVA_AUTHORIZE_URL}?client_id={}&response_type=code&redirect_uri={redirect_uri}&approval_prompt=force&scope=activity:read_all,profile:read_all",
                              config.client_id());

  eprintln!("\nOpen this URL in your browser to authorize the application:\n");
  eprintln!("  {authorize_url}\n");
  eprintln!("Waiting for authorization...");

  let code = wait_for_callback(&listener)?;

  log::info!("Authorization code received, exchanging for tokens");
  exchange_code_for_tokens(config, &code, &redirect_uri)
}

/// Wait for the Strava OAuth redirect to arrive on the local server.
/// Returns the authorization code from the query string.
fn wait_for_callback(listener: &TcpListener) -> Result<String, StravaError> {
  let (mut stream, _) =
    listener.accept()
            .map_err(|e| StravaError::OAuthError(format!("Failed to accept connection: {e}")))?;

  let mut reader = BufReader::new(&stream);
  let mut request_line = String::new();
  reader.read_line(&mut request_line)
        .map_err(|e| StravaError::OAuthError(format!("Failed to read request: {e}")))?;

  // Parse: "GET /?code=XXX&scope=YYY HTTP/1.1"
  let path = request_line.split_whitespace()
                         .nth(1)
                         .ok_or_else(|| StravaError::OAuthError("Malformed HTTP request".into()))?;

  let query = path.split_once('?').map(|(_, q)| q).unwrap_or("");

  // Check for error (user denied authorization)
  if let Some(error) = extract_query_param(query, "error") {
    let response = format!("HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n{ERROR_HTML}");
    let _ = stream.write_all(response.as_bytes());
    return Err(StravaError::OAuthError(format!("Authorization denied: {error}")));
  }

  let code = extract_query_param(query, "code").ok_or_else(|| {
               let response =
                 format!("HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n{ERROR_HTML}");
               let _ = stream.write_all(response.as_bytes());
               StravaError::OAuthError("No authorization code in redirect".into())
             })?;

  let response = format!("HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n{SUCCESS_HTML}");
  let _ = stream.write_all(response.as_bytes());

  Ok(code)
}

/// Extract a query parameter value from a query string like
/// "code=XXX&scope=YYY".
fn extract_query_param(query: &str, key: &str) -> Option<String> {
  query.split('&').find_map(|pair| {
                    let (k, v) = pair.split_once('=')?;
                    if k == key { Some(v.to_string()) } else { None }
                  })
}

/// Exchange the authorization code for an access token and refresh token.
fn exchange_code_for_tokens(config: &StravaConfig,
                            code: &str,
                            redirect_uri: &str)
                            -> Result<TokenResponse, StravaError> {
  let client = ReqwestClient::new();

  let response =
    client.post(STRAVA_TOKEN_URL)
          .form(&[("client_id", config.client_id()),
                  ("client_secret", config.client_secret()),
                  ("code", code),
                  ("grant_type", "authorization_code"),
                  ("redirect_uri", redirect_uri)])
          .send()
          .map_err(|e| StravaError::OAuthError(format!("Token exchange request failed: {e}")))?;

  if !response.status().is_success() {
    let status = response.status();
    let body = response.text().unwrap_or_default();
    return Err(StravaError::OAuthError(format!("Token exchange failed with status {status}: {body}")));
  }

  let body =
    response.text()
            .map_err(|e| StravaError::OAuthError(format!("Failed to read token response: {e}")))?;

  let token_response: TokenResponse = serde_json::from_str(&body).map_err(|e| {
    StravaError::OAuthError(format!("Failed to parse token response: {e} — body: {body}"))
  })?;

  log::info!("Token exchange successful");
  Ok(token_response)
}
