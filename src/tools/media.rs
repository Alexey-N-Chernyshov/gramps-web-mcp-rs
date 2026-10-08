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

//! Client-provided media bytes: create-from-bytes, in-place binary replacement, and the
//! size-limited binary readback shared by both.

use std::fmt;

use md5::{Digest as _, Md5};
use sha2::Sha256;

use crate::{
    client::{Error, GrampsClient, Result},
    models::Handle,
    tools::{create, get},
};

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub(crate) fn compute_md5_hex(bytes: &[u8]) -> String {
    let mut hasher = Md5::new();
    hasher.update(bytes);
    to_hex(&hasher.finalize())
}

pub(crate) fn compute_sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    to_hex(&hasher.finalize())
}

fn normalize_mime(mime: &str) -> String {
    mime.split(';')
        .next()
        .unwrap_or(mime)
        .trim()
        .to_ascii_lowercase()
}

/// Resolves the MIME type to store, cross-checking the caller's claim against the file's
/// actual content (via magic-byte sniffing) so a mislabeled upload (e.g. an HTML error page
/// saved as "image/jpeg") is rejected instead of silently corrupting the media record.
pub(crate) fn resolve_mime(bytes: &[u8], caller_mime: Option<&str>) -> Result<String> {
    let sniffed = infer::get(bytes).map(|t| t.mime_type().to_string());
    match (caller_mime, sniffed) {
        (None, Some(sniffed_mime)) => Ok(sniffed_mime),
        (None, None) => Ok("application/octet-stream".to_string()),
        (Some(claimed), None) => Ok(normalize_mime(claimed)),
        (Some(claimed), Some(sniffed_mime)) => {
            let claimed_norm = normalize_mime(claimed);
            if claimed_norm == sniffed_mime {
                Ok(claimed_norm)
            } else {
                Err(Error::Validation(format!(
                    "declared MIME type \"{claimed}\" does not match the file's actual \
                     content, which looks like \"{sniffed_mime}\""
                )))
            }
        }
    }
}

pub(crate) fn check_size(bytes: &[u8], limit: u64) -> Result<()> {
    let len = bytes.len() as u64;
    if len > limit {
        return Err(Error::Validation(format!(
            "file is {len} bytes, which exceeds the configured limit of {limit} bytes"
        )));
    }
    Ok(())
}

/// A media write that may have partially succeeded: the binary may already be stored even
/// though metadata could not be applied, or vice versa. Callers get the handle (when one
/// exists) plus the underlying error so the record can be inspected or retried instead of
/// being silently lost.
#[derive(Debug)]
pub struct MediaUploadError {
    pub handle: Option<Handle>,
    pub source: Error,
}

impl fmt::Display for MediaUploadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.handle {
            Some(handle) => write!(
                f,
                "media {handle}: {}. Call update_media with this handle to fix metadata, \
                 or get_media_file to inspect what was actually saved.",
                self.source
            ),
            None => write!(f, "{}", self.source),
        }
    }
}

impl std::error::Error for MediaUploadError {}

impl From<Error> for MediaUploadError {
    fn from(source: Error) -> Self {
        Self {
            handle: None,
            source,
        }
    }
}

/// Creates a media record from client-provided bytes. Privacy, description and MIME are set
/// before the object is ever visible as a plain record (no temporary public window).
pub async fn create_media_from_bytes(
    client: &GrampsClient,
    bytes: Vec<u8>,
    description: Option<&str>,
    mime: Option<&str>,
    is_private: Option<bool>,
) -> std::result::Result<Handle, MediaUploadError> {
    check_size(&bytes, client.max_media_bytes())?;
    let resolved_mime = resolve_mime(&bytes, mime)?;

    let resp: serde_json::Value = client
        .post_bytes("/api/media/", bytes, &resolved_mime)
        .await?;
    let handle = create::extract_handle(resp)?;

    if let Err(e) =
        apply_media_metadata(client, &handle, &resolved_mime, description, is_private).await
    {
        return Err(MediaUploadError {
            handle: Some(handle),
            source: e,
        });
    }
    Ok(handle)
}

async fn apply_media_metadata(
    client: &GrampsClient,
    handle: &str,
    mime: &str,
    description: Option<&str>,
    is_private: Option<bool>,
) -> Result<()> {
    let mut body = get::get_object_by_handle(client, "media", handle).await?;
    body["mime"] = serde_json::json!(mime);
    if let Some(desc) = description {
        body["desc"] = serde_json::json!(desc);
    }
    if let Some(private) = is_private {
        body["private"] = serde_json::json!(private);
    }
    client
        .put::<_, serde_json::Value>(&format!("/api/media/{handle}"), &body)
        .await?;
    Ok(())
}

/// Bytes read back from Gramps, plus the hashes needed to verify them against a local original.
#[derive(Debug)]
pub struct MediaFile {
    pub bytes: Vec<u8>,
    pub mime: String,
    pub size_bytes: u64,
    pub md5: String,
    pub sha256: String,
}

/// Size-limited binary readback. Private media is refused unless `allow_private` is set —
/// the Gramps Web API itself does not gate file download on the `private` flag, so this MCP
/// server owns that policy explicitly.
pub async fn get_media_file(
    client: &GrampsClient,
    handle: &str,
    max_bytes: Option<u64>,
    allow_private: bool,
) -> Result<MediaFile> {
    let metadata = get::get_object_by_handle(client, "media", handle).await?;

    let is_private = metadata["private"].as_bool().unwrap_or(false);
    if is_private && !allow_private {
        return Err(Error::Validation(format!(
            "media {handle} is private; pass allow_private=true to read its bytes"
        )));
    }

    let server_limit = client.max_media_bytes();
    let effective_limit = match max_bytes {
        Some(requested) if requested > server_limit => {
            return Err(Error::Validation(format!(
                "requested max_bytes ({requested}) exceeds the server limit of {server_limit} bytes"
            )));
        }
        Some(requested) => requested,
        None => server_limit,
    };

    let (bytes, content_type) = client
        .get_bytes(&format!("/api/media/{handle}/file"))
        .await?;
    let size_bytes = bytes.len() as u64;
    if size_bytes > effective_limit {
        return Err(Error::Validation(format!(
            "media {handle} is {size_bytes} bytes, which exceeds the limit of {effective_limit} bytes"
        )));
    }

    let mime = metadata["mime"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or(content_type)
        .unwrap_or_else(|| "application/octet-stream".to_string());

    Ok(MediaFile {
        md5: compute_md5_hex(&bytes),
        sha256: compute_sha256_hex(&bytes),
        size_bytes,
        bytes,
        mime,
    })
}

/// A digest of a just-written media file. Excludes the bytes themselves — the caller already
/// has them, since they are what was just uploaded.
#[derive(Debug)]
pub struct MediaReplaceSummary {
    pub mime: String,
    pub size_bytes: u64,
    pub md5: String,
    pub sha256: String,
}

/// Replaces the binary file of an existing Media object in place, preserving its handle,
/// `gramps_id`, privacy, description and attributes (Gramps only updates `checksum`/`path`/
/// `mime` on the existing object; it is never recreated). An `If-Match` precondition against
/// the current checksum is sent for optimistic concurrency.
pub async fn replace_media_file(
    client: &GrampsClient,
    handle: &str,
    bytes: Vec<u8>,
    mime: Option<&str>,
    expected_md5: Option<&str>,
    expected_sha256: Option<&str>,
) -> std::result::Result<MediaReplaceSummary, MediaUploadError> {
    let wrap = |e: Error| MediaUploadError {
        handle: Some(handle.to_string()),
        source: e,
    };

    check_size(&bytes, client.max_media_bytes()).map_err(wrap)?;
    let resolved_mime = resolve_mime(&bytes, mime).map_err(wrap)?;

    let md5 = compute_md5_hex(&bytes);
    let sha256 = compute_sha256_hex(&bytes);
    if let Some(expected) = expected_md5 {
        if !expected.eq_ignore_ascii_case(&md5) {
            return Err(wrap(Error::Validation(format!(
                "expected_md5 \"{expected}\" does not match the computed MD5 \"{md5}\" \
                 of the supplied bytes"
            ))));
        }
    }
    if let Some(expected) = expected_sha256 {
        if !expected.eq_ignore_ascii_case(&sha256) {
            return Err(wrap(Error::Validation(format!(
                "expected_sha256 \"{expected}\" does not match the computed SHA-256 \"{sha256}\" \
                 of the supplied bytes"
            ))));
        }
    }

    let metadata = get::get_object_by_handle(client, "media", handle)
        .await
        .map_err(wrap)?;
    let old_checksum = metadata["checksum"].as_str();
    let size_bytes = bytes.len() as u64;

    client
        .put_bytes::<serde_json::Value>(
            &format!("/api/media/{handle}/file"),
            bytes,
            &resolved_mime,
            old_checksum,
        )
        .await
        .map_err(wrap)?;

    Ok(MediaReplaceSummary {
        mime: resolved_mime,
        size_bytes,
        md5,
        sha256,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const JPEG_MAGIC: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0];
    const PDF_MAGIC: &[u8] = b"%PDF-1.4\n";
    const PNG_MAGIC: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

    #[test]
    fn resolve_mime_sniffs_jpeg_when_no_mime_given() {
        let bytes = [JPEG_MAGIC, b"rest of a fake jpeg"].concat();
        assert_eq!(resolve_mime(&bytes, None).unwrap(), "image/jpeg");
    }

    #[test]
    fn resolve_mime_sniffs_pdf_signature() {
        let bytes = [PDF_MAGIC, b"1 0 obj ..."].concat();
        assert_eq!(resolve_mime(&bytes, None).unwrap(), "application/pdf");
    }

    #[test]
    fn resolve_mime_rejects_html_claiming_to_be_jpeg() {
        let bytes = b"<html><body>Access denied</body></html>".to_vec();
        let err = resolve_mime(&bytes, Some("image/jpeg")).unwrap_err();
        assert!(matches!(err, Error::Validation(_)), "got: {err:?}");
    }

    #[test]
    fn resolve_mime_accepts_matching_claim() {
        let bytes = [JPEG_MAGIC, b"rest"].concat();
        assert_eq!(
            resolve_mime(&bytes, Some("image/jpeg")).unwrap(),
            "image/jpeg"
        );
    }

    #[test]
    fn resolve_mime_trusts_claim_for_unrecognized_content() {
        let bytes = b"just some opaque bytes, not a known signature".to_vec();
        assert_eq!(
            resolve_mime(&bytes, Some("application/octet-stream")).unwrap(),
            "application/octet-stream"
        );
    }

    #[test]
    fn resolve_mime_rejects_generic_claim_contradicted_by_sniffed_content() {
        let bytes = [PNG_MAGIC, b"rest of a fake png"].concat();
        let err = resolve_mime(&bytes, Some("application/octet-stream")).unwrap_err();
        assert!(matches!(err, Error::Validation(_)), "got: {err:?}");
    }

    #[test]
    fn md5_and_sha256_match_known_digests() {
        // echo -n "gramps-mcp-rs" | md5sum / sha256sum
        let input = b"gramps-mcp-rs";
        assert_eq!(compute_md5_hex(input), "c86b8efedbf7b523586d6b58c4ca7b42");
        assert_eq!(
            compute_sha256_hex(input),
            "761c0659e35a2551b1f014f859f58948e1435bee9244e251db1bf0ab281200ce"
        );
    }

    #[test]
    fn check_size_rejects_oversized_input() {
        let bytes = vec![0u8; 10];
        let err = check_size(&bytes, 5).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("10"),
            "message should mention actual size: {msg}"
        );
        assert!(msg.contains('5'), "message should mention the limit: {msg}");
    }

    #[test]
    fn check_size_accepts_input_within_limit() {
        let bytes = vec![0u8; 5];
        assert!(check_size(&bytes, 5).is_ok());
    }
}
