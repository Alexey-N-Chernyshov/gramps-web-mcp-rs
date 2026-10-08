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

use gramps_web_mcp_rs::{client::GrampsClient, config::Config};
use std::time::Duration;
use testcontainers::{
    compose::DockerCompose,
    core::{wait::HttpWaitStrategy, IntoContainerPort, WaitFor},
};

const COMPOSE_FILE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/docker-compose.test.yml");
const SERVICE: &str = "grampsweb-test";
const PORT: u16 = 5000;
pub const TEST_USER: &str = "testadmin";
const TEST_PASS: &str = "Testpass1!";

pub struct TestFixture {
    pub _compose: DockerCompose,
    pub base_url: String,
    pub client: GrampsClient,
}

impl TestFixture {
    pub async fn new() -> Self {
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
            .expect("timed out waiting for the Gramps Web compose stack to start")
            .expect("failed to start the Gramps Web compose stack");

        let service = compose
            .service(SERVICE)
            .expect("grampsweb-test service should be running");
        let port = service.get_host_port_ipv4(PORT).await.unwrap();
        let base_url = format!("http://localhost:{port}");

        register_admin(&base_url).await;

        let client = GrampsClient::new(
            Config {
                gramps_api_url: base_url.clone(),
                gramps_username: TEST_USER.to_string(),
                gramps_password: TEST_PASS.to_string(),
                gramps_readonly: false,
                mcp_transport: Default::default(),
                mcp_http_host: Default::default(),
                mcp_http_port: Default::default(),
                mcp_auth_token: None,
                mcp_allowed_hosts: None,
                mcp_keep_alive: 300,
            },
            reqwest::Client::new(),
        );

        Self {
            _compose: compose,
            base_url,
            client,
        }
    }
}

async fn register_admin(base_url: &str) {
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
        .post(format!("{base_url}/api/users/{TEST_USER}/create_owner/"))
        .bearer_auth(setup_token)
        .json(&serde_json::json!({
            "password": TEST_PASS,
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
