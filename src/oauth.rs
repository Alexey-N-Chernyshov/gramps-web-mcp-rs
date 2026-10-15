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

//! Per-user OAuth 2.1 authorization for the HTTP transport (opt-in via
//! `MCP_OAUTH_ENABLED`): Authorization Code + PKCE + Dynamic Client
//! Registration, with Gramps Web's own `/api/token/` as the credential
//! check. See `docs`/the project plan for the full design — in short, this
//! module is a self-contained axum sub-app (discovery + DCR + login +
//! token issuance) plus one middleware that attaches a per-request
//! `GrampsClient` for whichever user is attached to the request's
//! extensions, which `GrampsMcpServer::client_for` (in the bin crate) reads
//! back out via rmcp's `RequestContext`.

mod authorize;
mod metadata;
mod register;
pub mod session;
pub mod store;
mod token;

use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use url::Url;

pub use store::Store;

pub struct OAuthState {
    pub store: Arc<Store>,
    /// The Gramps Web API base URL.
    pub api_url: Url,
    pub http: reqwest::Client,
    /// This server's own externally-visible base URL (`MCP_OAUTH_PUBLIC_URL`),
    /// used to build the discovery documents' endpoint URLs.
    pub issuer: Url,
    pub max_media_bytes: u64,
}

/// The unauthenticated OAuth endpoints: discovery, DCR, login, token
/// issuance. Mount this alongside (not behind) the `/mcp` route.
pub fn router(state: Arc<OAuthState>) -> Router {
    Router::new()
        .route(
            "/.well-known/oauth-protected-resource",
            get(metadata::protected_resource),
        )
        .route(
            "/.well-known/oauth-authorization-server",
            get(metadata::authorization_server),
        )
        .route("/register", post(register::register))
        .route(
            "/authorize",
            get(authorize::show_form).post(authorize::submit),
        )
        .route("/token", post(token::token))
        .with_state(state)
}

/// Resolves the incoming Bearer token to a per-user `GrampsClient` and
/// attaches it to the request's extensions. Apply this (instead of the
/// legacy static-token middleware) to the `/mcp` route when OAuth mode is
/// enabled.
pub async fn identity_middleware(
    State(state): State<Arc<OAuthState>>,
    mut req: Request,
    next: Next,
) -> Response {
    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::to_string);

    let Some(token) = token else {
        tracing::warn!("/mcp rejected: no Authorization header");
        return unauthorized(&state.issuer);
    };
    if state.store.session_for_access_token(&token).await.is_none() {
        tracing::warn!(
            token_prefix = &token[..token.len().min(8)],
            "/mcp rejected: unknown/expired access token"
        );
        return unauthorized(&state.issuer);
    }

    let token_source = Arc::new(session::OAuthSession::new(
        state.store.clone(),
        state.api_url.clone(),
        state.http.clone(),
        token,
    ));
    let client = crate::client::GrampsClient::new(
        state.api_url.clone(),
        token_source,
        state.max_media_bytes,
        state.http.clone(),
    );
    req.extensions_mut().insert(client);
    next.run(req).await
}

fn unauthorized(issuer: &Url) -> Response {
    let issuer = issuer.as_str().trim_end_matches('/');
    let header_value =
        format!("Bearer resource_metadata=\"{issuer}/.well-known/oauth-protected-resource\"");
    (
        StatusCode::UNAUTHORIZED,
        [(header::WWW_AUTHENTICATE, header_value)],
    )
        .into_response()
}
