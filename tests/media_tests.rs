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

//! Server-side-path and URL-download media creation. For client-provided bytes,
//! binary readback, and in-place replacement, see `media_binary_tests.rs`.

#[path = "support/helpers.rs"]
mod common;
#[path = "support/mcp.rs"]
mod mcp;

use common::TestFixture;
use mcp::{expect_created_handle, expect_error_text, expect_ok, expect_success, McpHarness};
use serde_json::json;

#[tokio::test]
async fn media_from_path_round_trip() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    let handle = expect_created_handle(
        harness
            .call(
                "create_media",
                json!({
                    "path": "/photos/test.jpg",
                    "description": "Test photo",
                    "mime": "image/jpeg",
                }),
            )
            .await,
    );

    let media = expect_ok(
        harness
            .call(
                "get_object",
                json!({"object_type": "media", "handle": handle}),
            )
            .await,
    );
    assert_eq!(media["handle"].as_str(), Some(handle.as_str()));
    assert_eq!(media["path"].as_str(), Some("/photos/test.jpg"));
    assert_eq!(media["desc"].as_str(), Some("Test photo"));

    expect_ok(
        harness
            .call(
                "update_media",
                json!({"handle": handle, "data": {"desc": "Updated photo"}}),
            )
            .await,
    );
    let updated = expect_ok(
        harness
            .call(
                "get_object",
                json!({"object_type": "media", "handle": handle}),
            )
            .await,
    );
    assert_eq!(updated["desc"].as_str(), Some("Updated photo"));

    expect_success(
        harness
            .call(
                "delete_object",
                json!({"object_type": "media", "handle": handle}),
            )
            .await,
    );
    let error = expect_error_text(
        harness
            .call(
                "get_object",
                json!({"object_type": "media", "handle": handle}),
            )
            .await,
    );
    assert!(error.contains("not found"), "got: {error}");
}

#[tokio::test]
async fn media_from_url_round_trip() {
    use tokio::io::AsyncWriteExt as _;

    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let file_url = format!("http://127.0.0.1:{port}/photo.jpg");

    tokio::spawn(async move {
        if let Ok((mut sock, _)) = listener.accept().await {
            let _ = sock
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type: image/jpeg\r\nContent-Length: 3\r\n\r\nABC",
                )
                .await;
        }
    });

    let handle = expect_created_handle(
        harness
            .call(
                "create_media",
                json!({"url": file_url, "description": "Downloaded photo"}),
            )
            .await,
    );

    let media = expect_ok(
        harness
            .call(
                "get_object",
                json!({"object_type": "media", "handle": handle}),
            )
            .await,
    );
    assert_eq!(media["handle"].as_str(), Some(handle.as_str()));
    assert_eq!(media["desc"].as_str(), Some("Downloaded photo"));
    assert_eq!(media["mime"].as_str(), Some("image/jpeg"));

    // FAILED: path/checksum empty after POST /api/media/ with raw bytes — confirms
    // the create-then-PUT-file two-step pattern is required, see gramps-web-api #189/#273.
    let path = media["path"].as_str().unwrap_or("");
    assert!(
        !path.is_empty(),
        "media[\"path\"] is empty after create_media (url) (got {:?}) — \
         file was not actually uploaded, only the metadata record was created",
        media["path"]
    );

    let checksum = media["checksum"].as_str().unwrap_or("");
    assert!(
        !checksum.is_empty(),
        "media[\"checksum\"] is empty after create_media (url) (got {:?}) — \
         file was not actually uploaded, only the metadata record was created",
        media["checksum"]
    );

    expect_ok(
        harness
            .call(
                "update_media",
                json!({"handle": handle, "data": {"desc": "Updated downloaded photo"}}),
            )
            .await,
    );
    let updated = expect_ok(
        harness
            .call(
                "get_object",
                json!({"object_type": "media", "handle": handle}),
            )
            .await,
    );
    assert_eq!(updated["desc"].as_str(), Some("Updated downloaded photo"));

    expect_success(
        harness
            .call(
                "delete_object",
                json!({"object_type": "media", "handle": handle}),
            )
            .await,
    );
    let error = expect_error_text(
        harness
            .call(
                "get_object",
                json!({"object_type": "media", "handle": handle}),
            )
            .await,
    );
    assert!(error.contains("not found"), "got: {error}");
}
