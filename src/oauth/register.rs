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

//! RFC 7591 Dynamic Client Registration — lets an MCP client (Claude Code,
//! Claude Desktop) obtain a `client_id` on the spot, with no human
//! pre-registering it anywhere.

use std::sync::Arc;

use axum::{extract::State, http::StatusCode, Json};
use serde::Deserialize;

use super::OAuthState;

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    #[serde(default)]
    pub redirect_uris: Vec<String>,
}

pub async fn register(
    State(state): State<Arc<OAuthState>>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    if req.redirect_uris.is_empty() {
        return Err(error_response(
            StatusCode::BAD_REQUEST,
            "invalid_client_metadata",
            "redirect_uris is required and must be non-empty",
        ));
    }

    let client_id = state
        .store
        .register_client(req.redirect_uris.clone())
        .await
        .map_err(|e| {
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "server_error",
                &e.to_string(),
            )
        })?;

    Ok(Json(serde_json::json!({
        "client_id": client_id,
        "redirect_uris": req.redirect_uris,
        "token_endpoint_auth_method": "none",
        "grant_types": ["authorization_code", "refresh_token"],
        "response_types": ["code"],
    })))
}

fn error_response(
    status: StatusCode,
    error: &str,
    description: &str,
) -> (StatusCode, Json<serde_json::Value>) {
    (
        status,
        Json(serde_json::json!({"error": error, "error_description": description})),
    )
}
