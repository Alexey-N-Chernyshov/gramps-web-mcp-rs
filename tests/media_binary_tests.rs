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

//! Client-provided media bytes against a real Gramps Web: create-from-bytes, binary
//! readback, and in-place replacement (GitHub issues #74, #75). Pre-flight checks that fire
//! before any request (invalid base64, size limit, MIME mismatch, checksum mismatch) live in
//! `mcp_server_tests.rs` instead — they don't need Docker. The "not found" path is
//! consolidated to one representative test here (see that file's module doc comment for the
//! "by handle" vs "by gramps_id" split).

#[path = "support/helpers.rs"]
mod common;
#[path = "support/mcp.rs"]
mod mcp;

use common::TestFixture;
use mcp::{expect_created_handle, expect_error_text, expect_media_file, expect_ok, McpHarness};
use serde_json::json;

fn fake_jpeg(payload: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0xFFu8, 0xD8, 0xFF, 0xE0];
    bytes.extend_from_slice(payload);
    bytes
}

fn fake_pdf(payload: &[u8]) -> Vec<u8> {
    let mut bytes = b"%PDF-1.4\n".to_vec();
    bytes.extend_from_slice(payload);
    bytes
}

fn base64_of(bytes: &[u8]) -> String {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

#[tokio::test]
async fn media_from_bytes_jpeg_round_trip() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    let bytes = fake_jpeg(b"fake jpeg payload");
    let handle = expect_created_handle(
        harness
            .call(
                "create_media",
                json!({"data_base64": base64_of(&bytes), "description": "Scanned photo"}),
            )
            .await,
    );

    let stored = expect_ok(
        harness
            .call(
                "get_object",
                json!({"object_type": "media", "handle": handle}),
            )
            .await,
    );
    assert_eq!(stored["handle"].as_str(), Some(handle.as_str()));
    assert_eq!(stored["desc"].as_str(), Some("Scanned photo"));
    assert_eq!(stored["mime"].as_str(), Some("image/jpeg"));
    assert!(!stored["path"].as_str().unwrap_or("").is_empty());
    let gramps_checksum = stored["checksum"].as_str().unwrap_or("").to_string();
    assert!(!gramps_checksum.is_empty());

    let (summary, file_bytes) = expect_media_file(
        harness
            .call("get_media_file", json!({"handle": handle}))
            .await,
    );
    assert_eq!(
        file_bytes, bytes,
        "readback bytes must exactly match the uploaded bytes"
    );
    assert_eq!(summary["mime"].as_str(), Some("image/jpeg"));
    assert_eq!(summary["size_bytes"].as_u64(), Some(bytes.len() as u64));
    assert_eq!(
        summary["md5"].as_str(),
        Some(gramps_checksum.as_str()),
        "our MD5 must agree with Gramps' own checksum field"
    );
}

#[tokio::test]
async fn media_from_bytes_pdf_round_trip() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    let bytes = fake_pdf(b"1 0 obj << >> endobj %%EOF");
    let handle = expect_created_handle(
        harness
            .call(
                "create_media",
                json!({
                    "data_base64": base64_of(&bytes),
                    "description": "Scanned document",
                    "mime": "application/pdf",
                }),
            )
            .await,
    );

    let stored = expect_ok(
        harness
            .call(
                "get_object",
                json!({"object_type": "media", "handle": handle}),
            )
            .await,
    );
    assert_eq!(stored["mime"].as_str(), Some("application/pdf"));

    let (summary, file_bytes) = expect_media_file(
        harness
            .call("get_media_file", json!({"handle": handle}))
            .await,
    );
    assert_eq!(file_bytes, bytes);
    assert_eq!(summary["mime"].as_str(), Some("application/pdf"));
}

#[tokio::test]
async fn media_from_bytes_sets_privacy_at_creation() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    let bytes = fake_jpeg(b"private photo");
    let handle = expect_created_handle(
        harness
            .call(
                "create_media",
                json!({"data_base64": base64_of(&bytes), "is_private": true}),
            )
            .await,
    );

    let stored = expect_ok(
        harness
            .call(
                "get_object",
                json!({"object_type": "media", "handle": handle}),
            )
            .await,
    );
    assert_eq!(
        stored["private"].as_bool(),
        Some(true),
        "privacy must be set at creation, with no public window"
    );
}

#[tokio::test]
async fn replace_media_file_preserves_handle_and_metadata() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    let original = fake_jpeg(b"original photo");
    let handle = expect_created_handle(
        harness
            .call(
                "create_media",
                json!({
                    "data_base64": base64_of(&original),
                    "description": "Family portrait",
                    "is_private": true,
                }),
            )
            .await,
    );

    let before = expect_ok(
        harness
            .call(
                "get_object",
                json!({"object_type": "media", "handle": handle}),
            )
            .await,
    );
    let gramps_id = before["gramps_id"].as_str().unwrap_or_default().to_string();

    let replacement = fake_pdf(b"replacement document");
    let summary = expect_ok(
        harness
            .call(
                "replace_media_file",
                json!({
                    "handle": handle,
                    "data_base64": base64_of(&replacement),
                    "mime": "application/pdf",
                }),
            )
            .await,
    );
    assert_eq!(summary["mime"].as_str(), Some("application/pdf"));
    assert_eq!(
        summary["size_bytes"].as_u64(),
        Some(replacement.len() as u64)
    );

    let after = expect_ok(
        harness
            .call(
                "get_object",
                json!({"object_type": "media", "handle": handle}),
            )
            .await,
    );
    assert_eq!(
        after["handle"].as_str(),
        Some(handle.as_str()),
        "handle must be stable"
    );
    assert_eq!(after["gramps_id"].as_str(), Some(gramps_id.as_str()));
    assert_eq!(
        after["desc"].as_str(),
        Some("Family portrait"),
        "description must be preserved"
    );
    assert_eq!(
        after["private"].as_bool(),
        Some(true),
        "privacy must be preserved"
    );
    assert_eq!(after["mime"].as_str(), Some("application/pdf"));
    assert_ne!(
        after["checksum"].as_str(),
        before["checksum"].as_str(),
        "checksum must reflect the new content"
    );

    let (file_summary, file_bytes) = expect_media_file(
        harness
            .call(
                "get_media_file",
                json!({"handle": handle, "allow_private": true}),
            )
            .await,
    );
    assert_eq!(
        file_bytes, replacement,
        "readback must match the replacement bytes exactly"
    );
    assert_eq!(file_summary["md5"].as_str(), summary["md5"].as_str());
    assert_eq!(file_summary["sha256"].as_str(), summary["sha256"].as_str());
}

#[tokio::test]
async fn get_media_file_private_without_flag_rejected() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    let bytes = fake_jpeg(b"secret photo");
    let handle = expect_created_handle(
        harness
            .call(
                "create_media",
                json!({"data_base64": base64_of(&bytes), "is_private": true}),
            )
            .await,
    );

    let error = expect_error_text(
        harness
            .call("get_media_file", json!({"handle": handle}))
            .await,
    );
    assert!(error.contains("private"), "got: {error}");
}

#[tokio::test]
async fn get_media_file_private_with_flag_allowed() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    let bytes = fake_jpeg(b"secret photo");
    let handle = expect_created_handle(
        harness
            .call(
                "create_media",
                json!({"data_base64": base64_of(&bytes), "is_private": true}),
            )
            .await,
    );

    let (_summary, file_bytes) = expect_media_file(
        harness
            .call(
                "get_media_file",
                json!({"handle": handle, "allow_private": true}),
            )
            .await,
    );
    assert_eq!(file_bytes, bytes);
}

#[tokio::test]
async fn get_media_file_exceeds_requested_max_bytes_rejected() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    let bytes = fake_jpeg(&[0u8; 100]);
    let handle = expect_created_handle(
        harness
            .call("create_media", json!({"data_base64": base64_of(&bytes)}))
            .await,
    );

    let error = expect_error_text(
        harness
            .call("get_media_file", json!({"handle": handle, "max_bytes": 10}))
            .await,
    );
    assert!(error.contains("exceeds"), "got: {error}");
}

#[tokio::test]
async fn get_media_file_unknown_handle_returns_not_found() {
    let fixture = TestFixture::new().await;
    let harness =
        McpHarness::connect(&fixture.base_url, &fixture.username, common::TEST_PASS).await;

    let error = expect_error_text(
        harness
            .call("get_media_file", json!({"handle": "NONEXISTENT_HANDLE"}))
            .await,
    );
    assert!(error.contains("not found"), "got: {error}");
}
