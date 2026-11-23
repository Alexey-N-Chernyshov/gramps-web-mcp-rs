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

//! RFC 9728 (protected resource metadata) and RFC 8414 (authorization server
//! metadata) discovery documents — how an MCP client finds out it needs to
//! authorize, and where.

use std::sync::Arc;

use axum::{extract::State, Json};

use super::OAuthState;

pub async fn protected_resource(State(state): State<Arc<OAuthState>>) -> Json<serde_json::Value> {
    let issuer = state.issuer.as_str().trim_end_matches('/');
    Json(serde_json::json!({
        "resource": issuer,
        "authorization_servers": [issuer],
    }))
}

pub async fn authorization_server(State(state): State<Arc<OAuthState>>) -> Json<serde_json::Value> {
    let issuer = state.issuer.as_str().trim_end_matches('/');
    Json(serde_json::json!({
        "issuer": issuer,
        "authorization_endpoint": format!("{issuer}/authorize"),
        "token_endpoint": format!("{issuer}/token"),
        "registration_endpoint": format!("{issuer}/register"),
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code", "refresh_token"],
        "code_challenge_methods_supported": ["S256"],
        "token_endpoint_auth_methods_supported": ["none"],
    }))
}
