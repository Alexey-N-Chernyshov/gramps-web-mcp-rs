use gramps_mcp_rs::{client::GrampsClient, config::Config};
use std::time::Duration;
use testcontainers::{
    core::{wait::HttpWaitStrategy, IntoContainerPort, WaitFor},
    runners::AsyncRunner,
    ContainerAsync, GenericImage, ImageExt,
};

const IMAGE: &str = "ghcr.io/gramps-project/grampsweb";
const TAG: &str = "latest";
const PORT: u16 = 5000;
pub const TEST_USER: &str = "testadmin";
const TEST_PASS: &str = "Testpass1!";

pub struct TestFixture {
    pub _container: ContainerAsync<GenericImage>,
    pub base_url: String,
    pub client: GrampsClient,
}

impl TestFixture {
    pub async fn new() -> Self {
        let container: ContainerAsync<GenericImage> = GenericImage::new(IMAGE, TAG)
            .with_exposed_port(PORT.tcp())
            .with_wait_for(WaitFor::http(
                HttpWaitStrategy::new("/api/metadata/")
                    .with_port(PORT.tcp())
                    .with_response_matcher(|_| true),
            ))
            .with_env_var("GRAMPSWEB_TREE", "testdb")
            .with_env_var("GRAMPSWEB_SECRET_KEY", "integration-test-secret")
            .with_startup_timeout(Duration::from_secs(120))
            .start()
            .await
            .expect("failed to start Gramps Web container");

        let port = container.get_host_port_ipv4(PORT).await.unwrap();
        let base_url = format!("http://localhost:{port}");

        register_admin(&base_url).await;

        let client = GrampsClient::new(
            Config {
                gramps_api_url: base_url.clone(),
                gramps_username: TEST_USER.to_string(),
                gramps_password: TEST_PASS.to_string(),
                gramps_readonly: false,
            },
            reqwest::Client::new(),
        );

        Self {
            _container: container,
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
