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

//! Every other integration test runs against `docker-compose.test.yml`'s multi-tree
//! (`GRAMPSWEB_TREE: "*"`) server — a fresh Gramps Web *tree* per test, not a fresh
//! *server*, for speed (see `tests/support/helpers.rs`). This is the one test that still
//! runs single-tree mode (`GRAMPSWEB_TREE: testdb`, the mode every real deployment of this
//! MCP server actually runs in), in its own throwaway container from its own compose file,
//! so a behavioral difference between the two modes doesn't go unnoticed just because
//! nothing exercises single-tree mode at all anymore.

use gramps_web_mcp_rs::{
    auth::AuthManager,
    client::GrampsClient,
    models::person::{CreatePersonRequest, PersonName},
    tools::{create, get},
};
use std::{sync::Arc, time::Duration};
use testcontainers::{
    compose::DockerCompose,
    core::{wait::HttpWaitStrategy, IntoContainerPort, WaitFor},
};

const COMPOSE_FILE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docker-compose.test-singletree.yml"
);
const SERVICE: &str = "grampsweb-test-singletree";
const PORT: u16 = 5000;
const USER: &str = "testadmin";
const PASS: &str = "Testpass1!";

#[tokio::test]
async fn login_write_and_read_round_trip_in_single_tree_mode() {
    let mut compose = DockerCompose::with_local_client(&[COMPOSE_FILE]).with_wait_for_service(
        SERVICE,
        WaitFor::http(
            HttpWaitStrategy::new("/api/metadata/")
                .with_port(PORT.tcp())
                .with_response_matcher(|_| true),
        ),
    );

    tokio::time::timeout(Duration::from_secs(240), compose.up())
        .await
        .expect("timed out waiting for the single-tree Gramps Web stack to start")
        .expect("failed to start the single-tree Gramps Web compose stack");

    let service = compose
        .service(SERVICE)
        .expect("grampsweb-test-singletree service should be running");
    let port = service
        .get_host_port_ipv4(PORT)
        .await
        .expect("grampsweb-test-singletree service should expose its port");
    let base_url = format!("http://localhost:{port}");

    register_owner(&base_url).await;

    let api_url: url::Url = base_url
        .parse()
        .expect("test container URL should be valid");
    let http = reqwest::Client::new();
    let auth = AuthManager::new(
        api_url.clone(),
        USER.to_string(),
        PASS.to_string().into(),
        http.clone(),
    );
    let client = GrampsClient::new(api_url, Arc::new(auth), 50 * 1024 * 1024, http);

    // Login + a plain read.
    get::get_tree_info(&client)
        .await
        .expect("get_tree_info should succeed in single-tree mode");

    // One write, one read, round-tripped.
    let handle = create::create_person(
        &client,
        CreatePersonRequest {
            primary_name: Some(PersonName {
                first_name: Some("Singletree".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .await
    .expect("creating a person should succeed in single-tree mode");

    let person = get::get_object_by_handle(&client, "people", &handle, get::ReadOptions::default())
        .await
        .expect("reading the created person back should succeed in single-tree mode");
    assert_eq!(
        person["primary_name"]["first_name"].as_str(),
        Some("Singletree")
    );
}

async fn register_owner(base_url: &str) {
    let http = reqwest::Client::new();

    let token_resp = http
        .get(format!("{base_url}/api/token/create_owner/"))
        .send()
        .await
        .expect("failed to reach token/create_owner");
    let body: serde_json::Value = token_resp
        .json()
        .await
        .expect("failed to parse token response");
    let setup_token = body["access_token"]
        .as_str()
        .expect("no access_token in create_owner response");

    let resp = http
        .post(format!("{base_url}/api/users/{USER}/create_owner/"))
        .bearer_auth(setup_token)
        .json(&serde_json::json!({
            "password": PASS,
            "email": "test@example.com",
            "full_name": "Test Admin",
        }))
        .send()
        .await
        .expect("failed to create owner account");

    assert!(
        resp.status().is_success(),
        "create_owner failed: {}",
        resp.text().await.unwrap_or_default()
    );
}
