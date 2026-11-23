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

//! One Gramps Web instance for the whole test run, isolation per test via Gramps Web's own
//! multi-tree support (`GRAMPSWEB_TREE: "*"`, see `docker-compose.test.yml`): each
//! `TestFixture::new()` creates its own tree and its own tree-scoped user ("Owner" role,
//! i.e. role 4 — full read/write on that tree only, not site administration) rather than
//! spinning up a fresh container. The one-time site admin (created once, role 5) exists only
//! to call the tree/user-creation endpoints that require it; no test touches it directly.

use gramps_web_mcp_rs::{auth::AuthManager, client::GrampsClient};
use std::{
    sync::{atomic::AtomicU64, Arc, OnceLock},
    time::Duration,
};
use testcontainers::{
    compose::DockerCompose,
    core::{wait::HttpWaitStrategy, IntoContainerPort, WaitFor},
};

const COMPOSE_FILE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/docker-compose.test.yml");
const SERVICE: &str = "grampsweb-test";
const PORT: u16 = 5000;
// Fixed (not random) so a second local `cargo test` run reuses the still-running container
// from the first instead of piling up a new one — `docker compose up -d` against the same
// project name is a no-op once it's already up. Each test's isolation comes from its own
// Gramps Web tree (see `TestFixture::new`), not from a fresh container.
const PROJECT_NAME: &str = "gramps-mcp-rs-test";

// The one-time site admin, used internally by this module only (to create trees and
// tree-scoped users via the admin-only `/api/trees/`/`/api/users/` endpoints). No test
// talks to Gramps Web as this user.
const ADMIN_USER: &str = "testadmin";
const ADMIN_PASS: &str = "Testpass1!";

/// Shared password for every tree-scoped test user `TestFixture::new` creates. Only the
/// username needs to be unique per fixture — Gramps Web doesn't require unique passwords.
pub const TEST_PASS: &str = "Testpass1!";

pub struct TestFixture {
    // Not every test binary that includes this file via `#[path]` needs the base URL —
    // only the ones that build their own GrampsClient (or OAuth app) against it.
    #[allow(dead_code)]
    pub base_url: String,
    // Likewise, `oauth_tests.rs` only needs `base_url` and builds its own clients
    // through the OAuth flow rather than this pre-authenticated one.
    #[allow(dead_code)]
    pub client: GrampsClient,
    /// This fixture's tree-scoped user. Only `client_tests.rs` (to build a second client
    /// with a deliberately wrong password) and `oauth_tests.rs` (to log in through the OAuth
    /// form) need the username directly rather than going through `client`.
    #[allow(dead_code)]
    pub username: String,
}

impl TestFixture {
    pub async fn new() -> Self {
        let base_url = ensure_stack().to_string();
        ensure_admin(&base_url).await;

        let admin_token = admin_login(&base_url)
            .await
            .expect("admin login should succeed");
        let tree_id = create_tree(&base_url, &admin_token, &unique_name("tree")).await;
        let username = unique_name("tu");
        create_tree_user(&base_url, &admin_token, &username, &tree_id).await;

        let api_url: url::Url = base_url
            .parse()
            .expect("test container URL should be valid");
        let http = reqwest::Client::new();
        let auth = AuthManager::new(
            api_url.clone(),
            username.clone(),
            TEST_PASS.to_string().into(),
            http.clone(),
        );
        let client = GrampsClient::new(api_url, Arc::new(auth), 50 * 1024 * 1024, http);

        Self {
            base_url,
            client,
            username,
        }
    }

    /// Logs in as this fixture's tree user and mints a real persistent
    /// `sync`-scope access token, the way "Add sync device" does in the UI.
    /// `sync` allows several tokens per user, hence the `/tokens/` endpoint.
    #[allow(dead_code)]
    pub async fn create_sync_token(&self) -> String {
        let http = reqwest::Client::new();
        let resp = http
            .post(format!("{}/api/token/", self.base_url))
            .json(&serde_json::json!({"username": self.username, "password": TEST_PASS}))
            .send()
            .await
            .expect("failed to log in as the tree user to mint a sync token");
        assert!(
            resp.status().is_success(),
            "tree user login failed: {}",
            resp.text().await.unwrap_or_default()
        );
        let body: serde_json::Value = resp
            .json()
            .await
            .expect("failed to parse tree user login response");
        let jwt = body["access_token"]
            .as_str()
            .expect("no access_token in tree user login response");

        let resp = http
            .post(format!(
                "{}/api/users/-/access-tokens/sync/tokens/",
                self.base_url
            ))
            .bearer_auth(jwt)
            .json(&serde_json::json!({"label": "test-device"}))
            .send()
            .await
            .expect("failed to create a sync token");
        assert!(
            resp.status().is_success(),
            "sync token creation failed: {}",
            resp.text().await.unwrap_or_default()
        );
        let body: serde_json::Value = resp
            .json()
            .await
            .expect("failed to parse sync token creation response");
        body["token"]
            .as_str()
            .expect("no token in sync token creation response")
            .to_string()
    }
}

/// A client with arbitrary credentials/limits, for tests that only exercise pre-flight
/// validation (size limits, MIME/checksum mismatches) and never need a real backend —
/// `base_url` can safely point at an address nothing is listening on, so these credentials
/// are never actually checked.
///
/// Only `media_binary_tests.rs` uses this today; other test binaries including this file
/// via `#[path]` don't need it.
#[allow(dead_code)]
pub fn client_for_url(base_url: &str, max_media_bytes: u64) -> GrampsClient {
    let api_url: url::Url = base_url.parse().expect("test URL should be valid");
    let http = reqwest::Client::new();
    let auth = AuthManager::new(
        api_url.clone(),
        "unused".to_string(),
        "unused".to_string().into(),
        http.clone(),
    );
    GrampsClient::new(api_url, Arc::new(auth), max_media_bytes, http)
}

/// A name unique within this test binary's process, for tree display names and usernames
/// (plain alphanumerics — safe for both). Not unique across separate processes, but doesn't
/// need to be: either this process started its own fresh container (nothing else is using
/// it), or `GRAMPS_TEST_URL` points at one a CI workflow step just brought up for this job
/// alone (see `.github/workflows/coverage.yml`).
fn unique_name(prefix: &str) -> String {
    static NEXT_ID: AtomicU64 = AtomicU64::new(0);
    let n = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!("{prefix}{}n{n}", std::process::id())
}

/// The whole test run's one Gramps Web instance: `GRAMPS_TEST_URL` if set (a CI workflow
/// step brought it up — see `.github/workflows/coverage.yml`), otherwise a docker-compose
/// stack this process starts on first use. Calling this — i.e. calling `TestFixture::new` —
/// from more than one test is exactly the point: every test shares this one instance and
/// gets its own isolation via a Gramps Web tree instead of its own container.
fn ensure_stack() -> &'static str {
    static STACK_BASE_URL: OnceLock<String> = OnceLock::new();
    STACK_BASE_URL.get_or_init(|| {
        if let Ok(url) = std::env::var("GRAMPS_TEST_URL") {
            return url.trim_end_matches('/').to_string();
        }

        // `DockerCompose::up` is async and must run on *some* Tokio runtime, but it can't
        // run on the calling `#[tokio::test]`'s own runtime: that runtime is torn down the
        // moment that one test function returns, which would tear the container down with
        // it (it lives only as long as something keeps polling/holding the `DockerCompose`
        // guard). A dedicated thread with its own runtime, parked forever, keeps the guard
        // (and so the container) alive for the rest of the process instead.
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("gramps-test-stack".to_string())
            .spawn(move || {
                let rt = tokio::runtime::Runtime::new()
                    .expect("failed to build a Tokio runtime for the shared test stack");
                rt.block_on(start_and_park(tx));
            })
            .expect("failed to spawn the shared test stack thread");

        rx.recv()
            .expect("the shared test stack thread exited before reporting its status")
            .expect("failed to start the shared Gramps Web test stack")
    })
}

/// Starts the compose stack and reports its base URL back over `tx`, then parks forever —
/// see `ensure_stack` for why. Runs entirely on the dedicated thread/runtime `ensure_stack`
/// sets up; never called directly from a test.
async fn start_and_park(tx: std::sync::mpsc::Sender<Result<String, String>>) {
    let mut compose = DockerCompose::with_local_client(&[COMPOSE_FILE])
        .with_project_name(PROJECT_NAME)
        .with_wait_for_service(
            SERVICE,
            WaitFor::http(
                HttpWaitStrategy::new("/ready")
                    .with_port(PORT.tcp())
                    .with_expected_status_code(200u16),
            ),
        );

    let result: Result<String, String> = async {
        tokio::time::timeout(Duration::from_secs(240), compose.up())
            .await
            .map_err(|_| "timed out waiting for the Gramps Web compose stack to start".to_string())?
            .map_err(|e| format!("failed to start the Gramps Web compose stack: {e}"))?;
        let service = compose
            .service(SERVICE)
            .ok_or_else(|| "grampsweb-test service should be running".to_string())?;
        let port = service
            .get_host_port_ipv4(PORT)
            .await
            .map_err(|e| format!("grampsweb-test service should expose its port: {e}"))?;
        Ok(format!("http://localhost:{port}"))
    }
    .await;

    let started = result.is_ok();
    let _ = tx.send(result);
    if started {
        std::future::pending::<()>().await;
    }
}

/// Logs in as the shared site admin, returning its access token.
async fn admin_login(base_url: &str) -> Result<String, String> {
    let http = reqwest::Client::new();
    let resp = http
        .post(format!("{base_url}/api/token/"))
        .json(&serde_json::json!({"username": ADMIN_USER, "password": ADMIN_PASS}))
        .send()
        .await
        .map_err(|e| format!("failed to reach /api/token/: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!(
            "admin login failed: {} {}",
            resp.status(),
            resp.text().await.unwrap_or_default()
        ));
    }
    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("failed to parse admin login response: {e}"))?;
    body["access_token"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| "no access_token in admin login response".to_string())
}

/// Ensures the shared site admin exists, tolerating the case where a previous run against
/// this same (intentionally reused, see `ensure_stack`) container already created it. Also
/// where `GRAMPS_TEST_URL`'s stack readiness is waited for (see `wait_ready`) — once, in the
/// same gate, before the first real request to it.
async fn ensure_admin(base_url: &str) {
    static ADMIN_READY: tokio::sync::OnceCell<()> = tokio::sync::OnceCell::const_new();
    ADMIN_READY
        .get_or_init(|| async {
            wait_ready(base_url).await;
            register_admin(base_url).await;
        })
        .await;
}

/// Polls Gramps Web's own unauthenticated readiness endpoint until it returns 200, for up to
/// 180s. The local (testcontainers) path doesn't need this — its own wait strategy (see
/// `start_and_park`) already confirmed readiness before `ensure_stack` returns — but
/// `GRAMPS_TEST_URL` names a stack that `coverage.yml` starts in the background and doesn't
/// wait for at all, relying on this instead.
async fn wait_ready(base_url: &str) {
    let http = reqwest::Client::new();
    let deadline = std::time::Instant::now() + Duration::from_secs(180);
    loop {
        if let Ok(resp) = http.get(format!("{base_url}/ready")).send().await {
            if resp.status().is_success() {
                return;
            }
        }
        if std::time::Instant::now() >= deadline {
            panic!(
                "Gramps Web at {base_url} did not become ready (GET /ready never returned \
                 200) within 180 seconds"
            );
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

async fn register_admin(base_url: &str) {
    let http = reqwest::Client::new();

    let token_resp = http
        .get(format!("{base_url}/api/token/create_owner/"))
        .send()
        .await
        .expect("failed to reach token/create_owner");

    if !token_resp.status().is_success() {
        // Most likely: a leftover admin from a previous run against this same reused
        // container. Confirm that's really the case instead of assuming it.
        let login = admin_login(base_url).await;
        assert!(
            login.is_ok(),
            "token/create_owner failed ({}) and logging in as the existing admin also \
             failed: {:?}",
            token_resp.status(),
            login.err()
        );
        return;
    }

    let body: serde_json::Value = token_resp
        .json()
        .await
        .expect("failed to parse token response");
    let setup_token = body["access_token"]
        .as_str()
        .expect("no access_token in create_owner response");

    let resp = http
        .post(format!("{base_url}/api/users/{ADMIN_USER}/create_owner/"))
        .bearer_auth(setup_token)
        .json(&serde_json::json!({
            "password": ADMIN_PASS,
            "email": "test-admin@example.com",
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

/// Creates a new, empty Gramps Web tree, returning its id.
async fn create_tree(base_url: &str, admin_token: &str, name: &str) -> String {
    let http = reqwest::Client::new();
    let resp = http
        .post(format!("{base_url}/api/trees/"))
        .bearer_auth(admin_token)
        .json(&serde_json::json!({"name": name}))
        .send()
        .await
        .expect("failed to create a test tree");
    assert!(
        resp.status().is_success(),
        "tree creation failed: {}",
        resp.text().await.unwrap_or_default()
    );
    let body: serde_json::Value = resp
        .json()
        .await
        .expect("failed to parse tree creation response");
    body["id"]
        .as_str()
        .expect("no id in tree creation response")
        .to_string()
}

/// Creates a user scoped to `tree_id` with role 4 ("Owner" — full read/write on that tree,
/// not site administration; see the module doc comment) and [`TEST_PASS`] as its password.
async fn create_tree_user(base_url: &str, admin_token: &str, username: &str, tree_id: &str) {
    let http = reqwest::Client::new();
    let resp = http
        .post(format!("{base_url}/api/users/{username}/"))
        .bearer_auth(admin_token)
        .json(&serde_json::json!({
            "password": TEST_PASS,
            "email": format!("{username}@example.com"),
            "full_name": username,
            "role": 4,
            "tree": tree_id,
        }))
        .send()
        .await
        .expect("failed to create a test tree user");
    assert!(
        resp.status().is_success(),
        "tree user creation failed: {}",
        resp.text().await.unwrap_or_default()
    );
}
