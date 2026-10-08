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

//! Client-provided media bytes: create-from-bytes, binary readback, and in-place
//! replacement (GitHub issues #74, #75).

#[path = "support/helpers.rs"]
mod common;

use gramps_web_mcp_rs::{
    client::Error,
    tools::{delete, get, media},
};

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

fn fake_png(payload: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0x89u8, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    bytes.extend_from_slice(payload);
    bytes
}

#[tokio::test]
async fn media_from_bytes_jpeg_round_trip() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let bytes = fake_jpeg(b"fake jpeg payload");

    let handle =
        media::create_media_from_bytes(client, bytes.clone(), Some("Scanned photo"), None, None)
            .await
            .unwrap();

    let stored = get::get_object_by_handle(client, "media", &handle)
        .await
        .unwrap();
    assert_eq!(stored["handle"].as_str(), Some(handle.as_str()));
    assert_eq!(stored["desc"].as_str(), Some("Scanned photo"));
    assert_eq!(stored["mime"].as_str(), Some("image/jpeg"));
    assert!(!stored["path"].as_str().unwrap_or("").is_empty());
    let gramps_checksum = stored["checksum"].as_str().unwrap_or("").to_string();
    assert!(!gramps_checksum.is_empty());

    let file = media::get_media_file(client, &handle, None, false)
        .await
        .unwrap();
    assert_eq!(
        file.bytes, bytes,
        "readback bytes must exactly match the uploaded bytes"
    );
    assert_eq!(file.mime, "image/jpeg");
    assert_eq!(file.size_bytes, bytes.len() as u64);
    assert_eq!(
        file.md5, gramps_checksum,
        "our MD5 must agree with Gramps' own checksum field"
    );

    delete::delete_object(client, "media", &handle)
        .await
        .unwrap();
}

#[tokio::test]
async fn media_from_bytes_pdf_round_trip() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let bytes = fake_pdf(b"1 0 obj << >> endobj %%EOF");

    let handle = media::create_media_from_bytes(
        client,
        bytes.clone(),
        Some("Scanned document"),
        Some("application/pdf"),
        None,
    )
    .await
    .unwrap();

    let stored = get::get_object_by_handle(client, "media", &handle)
        .await
        .unwrap();
    assert_eq!(stored["mime"].as_str(), Some("application/pdf"));

    let file = media::get_media_file(client, &handle, None, false)
        .await
        .unwrap();
    assert_eq!(file.bytes, bytes);
    assert_eq!(file.mime, "application/pdf");

    delete::delete_object(client, "media", &handle)
        .await
        .unwrap();
}

#[tokio::test]
async fn media_from_bytes_sets_privacy_at_creation() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let bytes = fake_jpeg(b"private photo");

    let handle = media::create_media_from_bytes(client, bytes, None, None, Some(true))
        .await
        .unwrap();

    let stored = get::get_object_by_handle(client, "media", &handle)
        .await
        .unwrap();
    assert_eq!(
        stored["private"].as_bool(),
        Some(true),
        "privacy must be set at creation, with no public window"
    );

    delete::delete_object(client, "media", &handle)
        .await
        .unwrap();
}

#[tokio::test]
async fn media_from_bytes_mime_mismatch_rejected() {
    let client = common::client_for_url("http://127.0.0.1:1", 50 * 1024 * 1024);
    let png = fake_png(b"fake png payload");

    let err = media::create_media_from_bytes(&client, png, None, Some("image/jpeg"), None)
        .await
        .unwrap_err();
    assert!(
        err.handle.is_none(),
        "nothing should be created before the mismatch is caught"
    );
    assert!(matches!(err.source, Error::Validation(_)), "got: {err:?}");
}

#[tokio::test]
async fn media_from_bytes_size_limit_enforced() {
    let client = common::client_for_url("http://127.0.0.1:1", 10);
    let bytes = vec![0xFFu8; 11];

    let err = media::create_media_from_bytes(&client, bytes, None, None, None)
        .await
        .unwrap_err();
    assert!(err.handle.is_none());
    assert!(matches!(err.source, Error::Validation(_)), "got: {err:?}");
}

#[tokio::test]
async fn media_from_bytes_metadata_failure_returns_recoverable_handle() {
    use axum::{
        extract::Path,
        http::StatusCode,
        routing::{get, post},
        Json, Router,
    };

    async fn token() -> Json<serde_json::Value> {
        Json(serde_json::json!({"access_token": "faketoken"}))
    }
    async fn create_media_handler() -> (StatusCode, Json<serde_json::Value>) {
        (
            StatusCode::CREATED,
            Json(serde_json::json!({"handle": "fakehandle0001"})),
        )
    }
    async fn get_media_handler(Path(_handle): Path<String>) -> Json<serde_json::Value> {
        Json(serde_json::json!({
            "handle": "fakehandle0001",
            "checksum": "deadbeef",
            "path": "/fake/path.bin",
            "mime": "application/octet-stream",
            "desc": "",
            "private": false,
        }))
    }
    async fn put_media_handler() -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    let app = Router::new()
        .route("/api/token/", post(token))
        .route("/api/media/", post(create_media_handler))
        .route(
            "/api/media/{handle}",
            get(get_media_handler).put(put_media_handler),
        );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = common::client_for_url(&format!("http://{addr}"), 50 * 1024 * 1024);
    let bytes = vec![0xAAu8; 16];

    let err = media::create_media_from_bytes(&client, bytes, None, None, None)
        .await
        .unwrap_err();

    assert_eq!(
        err.handle.as_deref(),
        Some("fakehandle0001"),
        "the handle must be surfaced even though metadata application failed"
    );
    assert!(
        matches!(err.source, Error::Api { status: 500, .. }),
        "got: {err:?}"
    );
}

#[tokio::test]
async fn replace_media_file_preserves_handle_and_metadata() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let original = fake_jpeg(b"original photo");
    let handle =
        media::create_media_from_bytes(client, original, Some("Family portrait"), None, Some(true))
            .await
            .unwrap();

    let before = get::get_object_by_handle(client, "media", &handle)
        .await
        .unwrap();
    let gramps_id = before["gramps_id"].as_str().unwrap_or_default().to_string();

    let replacement = fake_pdf(b"replacement document");
    let summary = media::replace_media_file(
        client,
        &handle,
        replacement.clone(),
        Some("application/pdf"),
        None,
        None,
    )
    .await
    .unwrap();

    assert_eq!(summary.mime, "application/pdf");
    assert_eq!(summary.size_bytes, replacement.len() as u64);

    let after = get::get_object_by_handle(client, "media", &handle)
        .await
        .unwrap();
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

    let file = media::get_media_file(client, &handle, None, true)
        .await
        .unwrap();
    assert_eq!(
        file.bytes, replacement,
        "readback must match the replacement bytes exactly"
    );
    assert_eq!(file.md5, summary.md5);
    assert_eq!(file.sha256, summary.sha256);

    delete::delete_object(client, "media", &handle)
        .await
        .unwrap();
}

#[tokio::test]
async fn replace_media_file_mime_mismatch_rejected() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let original = fake_jpeg(b"original photo");
    let handle = media::create_media_from_bytes(client, original, None, None, None)
        .await
        .unwrap();
    let before = get::get_object_by_handle(client, "media", &handle)
        .await
        .unwrap();

    let actually_png = fake_png(b"actually a png");
    let err = media::replace_media_file(
        client,
        &handle,
        actually_png,
        Some("application/pdf"),
        None,
        None,
    )
    .await
    .unwrap_err();
    assert_eq!(err.handle.as_deref(), Some(handle.as_str()));
    assert!(matches!(err.source, Error::Validation(_)), "got: {err:?}");

    let after = get::get_object_by_handle(client, "media", &handle)
        .await
        .unwrap();
    assert_eq!(
        after["checksum"].as_str(),
        before["checksum"].as_str(),
        "the original file must be untouched after a rejected replacement"
    );

    delete::delete_object(client, "media", &handle)
        .await
        .unwrap();
}

#[tokio::test]
async fn replace_media_file_expected_checksum_mismatch_rejected() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let original = fake_jpeg(b"original photo");
    let handle = media::create_media_from_bytes(client, original, None, None, None)
        .await
        .unwrap();
    let before = get::get_object_by_handle(client, "media", &handle)
        .await
        .unwrap();

    let replacement = fake_jpeg(b"a different photo");
    let wrong_md5 = "0".repeat(32);
    let err = media::replace_media_file(client, &handle, replacement, None, Some(&wrong_md5), None)
        .await
        .unwrap_err();
    assert_eq!(err.handle.as_deref(), Some(handle.as_str()));
    assert!(matches!(err.source, Error::Validation(_)), "got: {err:?}");

    let after = get::get_object_by_handle(client, "media", &handle)
        .await
        .unwrap();
    assert_eq!(after["checksum"].as_str(), before["checksum"].as_str());

    delete::delete_object(client, "media", &handle)
        .await
        .unwrap();
}

#[tokio::test]
async fn replace_media_file_size_limit_enforced() {
    let client = common::client_for_url("http://127.0.0.1:1", 10);
    let bytes = vec![0xFFu8; 11];

    let err = media::replace_media_file(&client, "some-handle", bytes, None, None, None)
        .await
        .unwrap_err();
    assert_eq!(err.handle.as_deref(), Some("some-handle"));
    assert!(matches!(err.source, Error::Validation(_)), "got: {err:?}");
}

#[tokio::test]
async fn replace_media_file_nonexistent_handle_returns_not_found() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let bytes = fake_jpeg(b"photo");
    let err = media::replace_media_file(client, "NONEXISTENT_HANDLE", bytes, None, None, None)
        .await
        .unwrap_err();
    assert_eq!(err.handle.as_deref(), Some("NONEXISTENT_HANDLE"));
    assert!(matches!(err.source, Error::NotFound(_)), "got: {err:?}");
}

#[tokio::test]
async fn get_media_file_private_without_flag_rejected() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let bytes = fake_jpeg(b"secret photo");
    let handle = media::create_media_from_bytes(client, bytes, None, None, Some(true))
        .await
        .unwrap();

    let err = media::get_media_file(client, &handle, None, false)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Validation(_)), "got: {err:?}");

    delete::delete_object(client, "media", &handle)
        .await
        .unwrap();
}

#[tokio::test]
async fn get_media_file_private_with_flag_allowed() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let bytes = fake_jpeg(b"secret photo");
    let handle = media::create_media_from_bytes(client, bytes.clone(), None, None, Some(true))
        .await
        .unwrap();

    let file = media::get_media_file(client, &handle, None, true)
        .await
        .unwrap();
    assert_eq!(file.bytes, bytes);

    delete::delete_object(client, "media", &handle)
        .await
        .unwrap();
}

#[tokio::test]
async fn get_media_file_exceeds_requested_max_bytes_rejected() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let bytes = fake_jpeg(&[0u8; 100]);
    let handle = media::create_media_from_bytes(client, bytes, None, None, None)
        .await
        .unwrap();

    let err = media::get_media_file(client, &handle, Some(10), false)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Validation(_)), "got: {err:?}");

    delete::delete_object(client, "media", &handle)
        .await
        .unwrap();
}

#[tokio::test]
async fn get_media_file_nonexistent_handle_returns_not_found() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let err = media::get_media_file(client, "NONEXISTENT_HANDLE", None, false)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::NotFound(_)), "got: {err:?}");
}
