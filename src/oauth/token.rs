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

//! The `/token` endpoint: exchanges an authorization code (PKCE-verified)
//! for our own access+refresh token pair, or rotates a refresh token for a
//! fresh access token (OAuth 2.1 requires rotation for public clients).

use std::sync::Arc;

use axum::{extract::State, http::StatusCode, Form, Json};
use base64::Engine as _;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::OAuthState;

const ACCESS_TOKEN_TTL_MINUTES: i64 = 15;

// A flat (not internally-tagged) struct: `serde_urlencoded`, which axum's
// `Form` extractor uses, can't deserialize an internally-tagged enum from a
// flat key-value form body — so we dispatch on `grant_type` ourselves.
#[derive(Debug, Deserialize)]
pub struct TokenRequest {
    grant_type: String,
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    redirect_uri: Option<String>,
    #[serde(default)]
    code_verifier: Option<String>,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    client_id: Option<String>,
}

pub async fn token(
    State(state): State<Arc<OAuthState>>,
    Form(req): Form<TokenRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    tracing::info!(
        grant_type = req.grant_type,
        has_code = req.code.is_some(),
        has_redirect_uri = req.redirect_uri.is_some(),
        has_code_verifier = req.code_verifier.is_some(),
        has_refresh_token = req.refresh_token.is_some(),
        client_id = req.client_id.as_deref(),
        "/token request"
    );
    match req.grant_type.as_str() {
        "authorization_code" => {
            let (Some(code), Some(redirect_uri), Some(code_verifier)) =
                (req.code, req.redirect_uri, req.code_verifier)
            else {
                return Err(grant_error(
                    "code, redirect_uri, and code_verifier are required for grant_type=authorization_code",
                ));
            };
            authorization_code_grant(&state, &code, &redirect_uri, &code_verifier).await
        }
        "refresh_token" => {
            let Some(refresh_token) = req.refresh_token else {
                return Err(grant_error(
                    "refresh_token is required for grant_type=refresh_token",
                ));
            };
            refresh_token_grant(&state, &refresh_token).await
        }
        other => Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "unsupported_grant_type",
                "error_description": format!("unsupported grant_type: {other}"),
            })),
        )),
    }
}

async fn authorization_code_grant(
    state: &OAuthState,
    code: &str,
    redirect_uri: &str,
    code_verifier: &str,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let record = state
        .store
        .take_code(code)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| {
            tracing::warn!(code, "token rejected: code invalid/expired/already used");
            grant_error("the authorization code is invalid, expired, or already used")
        })?;

    if record.redirect_uri != redirect_uri {
        tracing::warn!(
            expected = record.redirect_uri,
            got = redirect_uri,
            "token rejected: redirect_uri mismatch"
        );
        return Err(grant_error(
            "redirect_uri does not match the authorization request",
        ));
    }
    if !verify_pkce(code_verifier, &record.code_challenge) {
        tracing::warn!("token rejected: PKCE verification failed");
        return Err(grant_error("code_verifier does not match code_challenge"));
    }

    let (access_token, refresh_token) = state
        .store
        .create_session(
            record.gramps_refresh_token,
            chrono::Duration::minutes(ACCESS_TOKEN_TTL_MINUTES),
        )
        .await
        .map_err(internal_error)?;

    Ok(success_response(access_token, refresh_token))
}

async fn refresh_token_grant(
    state: &OAuthState,
    refresh_token: &str,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let rotated = state
        .store
        .rotate_refresh_token(
            refresh_token,
            chrono::Duration::minutes(ACCESS_TOKEN_TTL_MINUTES),
        )
        .await
        .map_err(internal_error)?
        .ok_or_else(|| grant_error("refresh_token is invalid or already used"))?;

    Ok(success_response(rotated.0, rotated.1))
}

fn verify_pkce(verifier: &str, challenge: &str) -> bool {
    let digest = Sha256::digest(verifier.as_bytes());
    let computed = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest);
    computed == challenge
}

fn success_response(access_token: String, refresh_token: String) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "access_token": access_token,
        "token_type": "Bearer",
        "expires_in": ACCESS_TOKEN_TTL_MINUTES * 60,
        "refresh_token": refresh_token,
    }))
}

fn grant_error(description: &str) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({"error": "invalid_grant", "error_description": description})),
    )
}

fn internal_error(e: impl std::fmt::Display) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({"error": "server_error", "error_description": e.to_string()})),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_verifies_matching_pair() {
        // code_verifier "abc", S256 challenge computed the same way the client would.
        let verifier = "abc";
        let digest = Sha256::digest(verifier.as_bytes());
        let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest);
        assert!(verify_pkce(verifier, &challenge));
    }

    #[test]
    fn pkce_rejects_mismatched_pair() {
        assert!(!verify_pkce("abc", "not-the-right-challenge"));
    }
}
