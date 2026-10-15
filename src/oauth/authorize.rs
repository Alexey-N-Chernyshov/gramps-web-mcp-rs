// Copyright 2026 Alexey Chernyshov
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! The `/authorize` step of the Authorization Code + PKCE flow: a plain,
//! self-rendered HTML login form asking for the user's own Gramps Web
//! username/password. We never store the password — only the
//! `refresh_token` Gramps hands back once it's checked.

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;

use super::{store::AuthCode, OAuthState};

const CODE_TTL_SECONDS: i64 = 60;

#[derive(Debug, Deserialize)]
pub struct AuthorizeParams {
    client_id: String,
    redirect_uri: String,
    state: Option<String>,
    code_challenge: String,
    code_challenge_method: String,
    #[serde(default)]
    response_type: Option<String>,
}

pub async fn show_form(
    State(state): State<Arc<OAuthState>>,
    Query(params): Query<AuthorizeParams>,
) -> Response {
    if let Err(message) = validate(&state, &params).await {
        return bad_request(message);
    }
    Html(render_form(&params, None)).into_response()
}

#[derive(Debug, Deserialize)]
pub struct AuthorizeSubmission {
    client_id: String,
    redirect_uri: String,
    state: Option<String>,
    code_challenge: String,
    code_challenge_method: String,
    #[serde(default)]
    response_type: Option<String>,
    username: String,
    password: SecretString,
}

pub async fn submit(
    State(state): State<Arc<OAuthState>>,
    Form(form): Form<AuthorizeSubmission>,
) -> Response {
    let params = AuthorizeParams {
        client_id: form.client_id.clone(),
        redirect_uri: form.redirect_uri.clone(),
        state: form.state.clone(),
        code_challenge: form.code_challenge.clone(),
        code_challenge_method: form.code_challenge_method.clone(),
        response_type: form.response_type.clone(),
    };
    if let Err(message) = validate(&state, &params).await {
        return bad_request(message);
    }

    let refresh_token =
        match fetch_gramps_refresh_token(&state, &form.username, &form.password).await {
            Ok(token) => token,
            Err(message) => return Html(render_form(&params, Some(&message))).into_response(),
        };

    let code = match state
        .store
        .put_code(AuthCode {
            client_id: params.client_id.clone(),
            redirect_uri: params.redirect_uri.clone(),
            code_challenge: params.code_challenge.clone(),
            gramps_refresh_token: refresh_token,
            expires_at: chrono::Utc::now() + chrono::Duration::seconds(CODE_TTL_SECONDS),
        })
        .await
    {
        Ok(code) => code,
        Err(e) => {
            tracing::error!(%e, "failed to persist authorization code");
            return Html(render_form(
                &params,
                Some("Internal error, please try again."),
            ))
            .into_response();
        }
    };

    let mut redirect_to = format!(
        "{}?code={}",
        params.redirect_uri,
        urlencoding::encode(&code)
    );
    if let Some(s) = &params.state {
        redirect_to.push_str(&format!("&state={}", urlencoding::encode(s)));
    }
    Redirect::to(&redirect_to).into_response()
}

async fn validate(state: &OAuthState, params: &AuthorizeParams) -> Result<(), &'static str> {
    if params.response_type.as_deref().unwrap_or("code") != "code" {
        tracing::warn!(?params, "authorize rejected: bad response_type");
        return Err("only response_type=code is supported");
    }
    if params.code_challenge_method != "S256" {
        tracing::warn!(?params, "authorize rejected: bad code_challenge_method");
        return Err("only code_challenge_method=S256 is supported");
    }
    let Some(client) = state.store.get_client(&params.client_id).await else {
        tracing::warn!(?params, "authorize rejected: unknown client_id");
        return Err("unknown client_id — register via /register first");
    };
    if !redirect_uri_allowed(&client.redirect_uris, &params.redirect_uri) {
        tracing::warn!(
            ?params,
            registered = ?client.redirect_uris,
            "authorize rejected: redirect_uri not registered for this client"
        );
        return Err("redirect_uri is not registered for this client");
    }
    Ok(())
}

async fn fetch_gramps_refresh_token(
    state: &OAuthState,
    username: &str,
    password: &SecretString,
) -> Result<String, String> {
    let url = format!(
        "{}/api/token/",
        state.api_url.as_str().trim_end_matches('/')
    );
    let resp = state
        .http
        .post(&url)
        .json(&serde_json::json!({ "username": username, "password": password.expose_secret() }))
        .send()
        .await
        .map_err(|e| format!("could not reach Gramps Web: {e}"))?;

    if !resp.status().is_success() {
        return Err("Invalid username or password.".to_string());
    }
    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("unexpected response from Gramps Web: {e}"))?;
    body["refresh_token"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| "Gramps Web did not return a refresh token.".to_string())
}

/// RFC 8252 loopback clients (Claude Code) may register a redirect_uri on
/// one ephemeral port and present a different one at `/authorize` time, so
/// loopback matches ignore the port; everything else needs an exact match
/// (e.g. Claude Desktop's fixed `https://claude.ai/api/mcp/auth_callback`).
fn redirect_uri_allowed(registered: &[String], requested: &str) -> bool {
    if registered.iter().any(|r| r == requested) {
        return true;
    }
    let Some(req_path) = split_loopback(requested) else {
        return false;
    };
    registered
        .iter()
        .filter_map(|r| split_loopback(r))
        .any(|path| path == req_path)
}

/// Parses `http://<host>[:<port>]<path>` into `path` iff it's a loopback
/// address (`localhost` or `127.0.0.1`, treated as interchangeable per
/// RFC 8252); `None` for anything else (including https, since
/// loopback-without-TLS is the RFC 8252 case).
fn split_loopback(uri: &str) -> Option<&str> {
    let rest = uri.strip_prefix("http://")?;
    let (authority, path) = match rest.find('/') {
        Some(idx) => (&rest[..idx], &rest[idx..]),
        None => (rest, "/"),
    };
    let host = authority.split(':').next().unwrap_or(authority);
    (host == "localhost" || host == "127.0.0.1").then_some(path)
}

fn bad_request(message: &str) -> Response {
    (StatusCode::BAD_REQUEST, message.to_string()).into_response()
}

fn render_form(params: &AuthorizeParams, error: Option<&str>) -> String {
    let error_html = error
        .map(|e| format!(r#"<p class="error">{}</p>"#, html_escape(e)))
        .unwrap_or_default();
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Sign in to Gramps</title>
<style>
  body {{ font-family: system-ui, sans-serif; max-width: 360px; margin: 10vh auto; color: #222; }}
  h1 {{ font-size: 1.25rem; }}
  label {{ display: block; margin-top: 1rem; font-size: 0.9rem; }}
  input[type=text], input[type=password] {{ width: 100%; padding: 0.5rem; margin-top: 0.25rem; box-sizing: border-box; }}
  button {{ margin-top: 1.5rem; width: 100%; padding: 0.6rem; }}
  .error {{ color: #b00020; }}
</style>
</head>
<body>
<h1>Sign in to Gramps</h1>
{error_html}
<form method="post" action="/authorize">
  <input type="hidden" name="client_id" value="{client_id}">
  <input type="hidden" name="redirect_uri" value="{redirect_uri}">
  <input type="hidden" name="state" value="{state}">
  <input type="hidden" name="code_challenge" value="{code_challenge}">
  <input type="hidden" name="code_challenge_method" value="{code_challenge_method}">
  <input type="hidden" name="response_type" value="code">
  <label>Username<input type="text" name="username" autofocus required></label>
  <label>Password<input type="password" name="password" required></label>
  <button type="submit">Sign in</button>
</form>
</body>
</html>"#,
        client_id = html_escape(&params.client_id),
        redirect_uri = html_escape(&params.redirect_uri),
        state = html_escape(params.state.as_deref().unwrap_or("")),
        code_challenge = html_escape(&params.code_challenge),
        code_challenge_method = html_escape(&params.code_challenge_method),
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_matches_ignore_port() {
        let registered = vec!["http://localhost:51000/callback".to_string()];
        assert!(redirect_uri_allowed(
            &registered,
            "http://localhost:51999/callback"
        ));
        assert!(redirect_uri_allowed(
            &registered,
            "http://127.0.0.1:4000/callback"
        ));
    }

    #[test]
    fn non_loopback_requires_exact_match() {
        let registered = vec!["https://claude.ai/api/mcp/auth_callback".to_string()];
        assert!(redirect_uri_allowed(
            &registered,
            "https://claude.ai/api/mcp/auth_callback"
        ));
        assert!(!redirect_uri_allowed(
            &registered,
            "https://evil.example/api/mcp/auth_callback"
        ));
    }

    #[test]
    fn loopback_path_still_must_match() {
        let registered = vec!["http://localhost:51000/callback".to_string()];
        assert!(!redirect_uri_allowed(
            &registered,
            "http://localhost:4000/other"
        ));
    }

    #[test]
    fn html_escape_neutralizes_markup() {
        assert_eq!(
            html_escape(r#"<script>"x"</script>"#),
            "&lt;script&gt;&quot;x&quot;&lt;/script&gt;"
        );
    }
}
