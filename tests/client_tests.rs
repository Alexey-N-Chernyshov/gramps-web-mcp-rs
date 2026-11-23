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

#[path = "support/helpers.rs"]
mod common;

use std::sync::Arc;

use gramps_web_mcp_rs::{
    auth::{AuthManager, SyncTokenSource},
    client::{Error, GrampsClient},
    tools::{delete, get},
};

#[tokio::test]
async fn auth_fails_with_wrong_password() {
    let fixture = common::TestFixture::new().await;
    let api_url: url::Url = fixture.base_url.parse().expect("test URL should be valid");
    let http = reqwest::Client::new();
    let auth = AuthManager::new(
        api_url.clone(),
        fixture.username.clone(),
        "wrongpassword".to_string().into(),
        http.clone(),
    );
    let bad_client = GrampsClient::new(api_url, Arc::new(auth), 50 * 1024 * 1024, http);
    let result = get::get_tree_info(&bad_client).await;
    assert!(
        matches!(result, Err(Error::Auth(_))),
        "expected Auth error, got: {result:?}"
    );
}

/// `SyncTokenSource` against a real `gramps-web-api`, not the fake HTTP server the unit
/// tests in `src/auth.rs` use: a real persistent sync token, minted the way the Gramps Web
/// UI's Add sync device button would, must actually exchange for a working JWT via
/// `POST /api/token/sync/` and read real tree data with it.
#[tokio::test]
async fn sync_token_authenticates_against_real_gramps_web() {
    let fixture = common::TestFixture::new().await;
    let sync_token = fixture.create_sync_token().await;

    let api_url: url::Url = fixture.base_url.parse().expect("test URL should be valid");
    let http = reqwest::Client::new();
    let auth = SyncTokenSource::new(api_url.clone(), sync_token.into(), http.clone());
    let client = GrampsClient::new(api_url, Arc::new(auth), 50 * 1024 * 1024, http);

    let result = get::get_tree_info(&client).await;
    assert!(result.is_ok(), "expected success, got: {result:?}");
}

/// A revoked/unknown sync token must fail the exchange, not silently fall through.
#[tokio::test]
async fn sync_token_auth_fails_with_bogus_token() {
    let fixture = common::TestFixture::new().await;

    let api_url: url::Url = fixture.base_url.parse().expect("test URL should be valid");
    let http = reqwest::Client::new();
    let auth = SyncTokenSource::new(
        api_url.clone(),
        "not-a-real-token".to_string().into(),
        http.clone(),
    );
    let client = GrampsClient::new(api_url, Arc::new(auth), 50 * 1024 * 1024, http);

    let result = get::get_tree_info(&client).await;
    assert!(
        matches!(result, Err(Error::Auth(_))),
        "expected Auth error, got: {result:?}"
    );
}

#[tokio::test]
async fn get_nonexistent_returns_not_found() {
    let fixture = common::TestFixture::new().await;
    let result = get::get_object_by_handle(
        &fixture.client,
        "people",
        "NONEXISTENT_HANDLE",
        get::ReadOptions::default(),
    )
    .await;
    assert!(
        matches!(result, Err(Error::NotFound(_))),
        "expected NotFound, got: {result:?}"
    );
}

#[tokio::test]
async fn delete_nonexistent_returns_not_found() {
    let fixture = common::TestFixture::new().await;
    let result = delete::delete_object(&fixture.client, "people", "NONEXISTENT_HANDLE").await;
    assert!(
        matches!(result, Err(Error::NotFound(_))),
        "expected NotFound, got: {result:?}"
    );
}
