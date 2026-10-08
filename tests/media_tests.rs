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

use gramps_web_mcp_rs::{
    client::Error,
    tools::{create, delete, get, update},
};

#[tokio::test]
async fn media_from_path_round_trip() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let handle = create::create_media_from_path(
        client,
        "/photos/test.jpg",
        Some("Test photo"),
        Some("image/jpeg"),
    )
    .await
    .unwrap();

    let media = get::get_object_by_handle(client, "media", &handle)
        .await
        .unwrap();
    assert_eq!(media["handle"].as_str(), Some(handle.as_str()));
    assert_eq!(media["path"].as_str(), Some("/photos/test.jpg"));
    assert_eq!(media["desc"].as_str(), Some("Test photo"));

    let mut body = media.clone();
    body["desc"] = serde_json::json!("Updated photo");
    update::update_media(client, &handle, &body).await.unwrap();
    let updated = get::get_object_by_handle(client, "media", &handle)
        .await
        .unwrap();
    assert_eq!(updated["desc"].as_str(), Some("Updated photo"));

    delete::delete_object(client, "media", &handle)
        .await
        .unwrap();
    assert!(matches!(
        get::get_object_by_handle(client, "media", &handle).await,
        Err(Error::NotFound(_))
    ));
}

#[tokio::test]
async fn media_from_url_round_trip() {
    use tokio::io::AsyncWriteExt as _;

    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

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

    let handle = create::create_media_from_url(client, &file_url, Some("Downloaded photo"), None)
        .await
        .unwrap();

    let media = get::get_object_by_handle(client, "media", &handle)
        .await
        .unwrap();
    assert_eq!(media["handle"].as_str(), Some(handle.as_str()));
    assert_eq!(media["desc"].as_str(), Some("Downloaded photo"));
    assert_eq!(media["mime"].as_str(), Some("image/jpeg"));

    // FAILED: path/checksum empty after POST /api/media/ with raw bytes — confirms
    // the create-then-PUT-file two-step pattern is required, see gramps-web-api #189/#273.
    let path = media["path"].as_str().unwrap_or("");
    assert!(
        !path.is_empty(),
        "media[\"path\"] is empty after create_media_from_url (got {:?}) — \
         file was not actually uploaded, only the metadata record was created",
        media["path"]
    );

    let checksum = media["checksum"].as_str().unwrap_or("");
    assert!(
        !checksum.is_empty(),
        "media[\"checksum\"] is empty after create_media_from_url (got {:?}) — \
         file was not actually uploaded, only the metadata record was created",
        media["checksum"]
    );

    let mut body = media.clone();
    body["desc"] = serde_json::json!("Updated downloaded photo");
    update::update_media(client, &handle, &body).await.unwrap();
    let updated = get::get_object_by_handle(client, "media", &handle)
        .await
        .unwrap();
    assert_eq!(updated["desc"].as_str(), Some("Updated downloaded photo"));

    delete::delete_object(client, "media", &handle)
        .await
        .unwrap();
    assert!(matches!(
        get::get_object_by_handle(client, "media", &handle).await,
        Err(Error::NotFound(_))
    ));
}
