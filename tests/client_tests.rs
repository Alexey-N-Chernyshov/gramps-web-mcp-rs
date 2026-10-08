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

use gramps_web_mcp_rs::{
    client::{Error, GrampsClient},
    config::Config,
    tools::{delete, get},
};

#[tokio::test]
async fn auth_fails_with_wrong_password() {
    let fixture = common::TestFixture::new().await;
    let bad_client = GrampsClient::new(
        Config {
            gramps_api_url: fixture.base_url.clone(),
            gramps_username: common::TEST_USER.to_string(),
            gramps_password: "wrongpassword".to_string(),
            gramps_readonly: false,
            mcp_transport: Default::default(),
            mcp_http_host: Default::default(),
            mcp_http_port: Default::default(),
            mcp_auth_token: None,
            mcp_allowed_hosts: None,
            mcp_keep_alive: 300,
            mcp_max_media_bytes: 50 * 1024 * 1024,
        },
        reqwest::Client::new(),
    );
    let result = get::get_tree_info(&bad_client).await;
    assert!(
        matches!(result, Err(Error::Auth(_))),
        "expected Auth error, got: {result:?}"
    );
}

#[tokio::test]
async fn get_nonexistent_returns_not_found() {
    let fixture = common::TestFixture::new().await;
    let result = get::get_object_by_handle(&fixture.client, "people", "NONEXISTENT_HANDLE").await;
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
