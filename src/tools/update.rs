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

use crate::client::{Error, GrampsClient, Result};

/// Read-modify-write a top-level patch onto the object at `/api/{endpoint}/{handle}`.
///
/// Gramps Web's PUT replaces the object wholesale, so sending just the fields the caller
/// wants to change would silently drop everything else (citation_list, note_list, ...).
/// This fetches the current object first and overlays `data`'s top-level keys onto it
/// before writing it back. The merge is shallow by design: a list or nested object in
/// `data` replaces the corresponding value whole, rather than being merged recursively,
/// so callers get predictable behavior instead of surprising deep-merge semantics.
///
/// There is an inherent GET/PUT race: a concurrent write between the two could be lost.
fn validate_patch<'a>(
    data: &'a serde_json::Value,
    handle: &str,
) -> Result<&'a serde_json::Map<String, serde_json::Value>> {
    let patch = data.as_object().ok_or_else(|| {
        Error::Validation(
            "`data` must be a JSON object — pass only the fields you want to change".to_string(),
        )
    })?;
    if patch.is_empty() {
        return Err(Error::Validation(
            "`data` is empty — nothing to update. Pass only the fields you want to change; \
             use get_object first if you need to see the current values"
                .to_string(),
        ));
    }
    if let Some(patch_handle) = patch.get("handle") {
        if patch_handle.as_str() != Some(handle) {
            return Err(Error::Validation(format!(
                "`data.handle` ({patch_handle}) does not match the `handle` argument \
                 (\"{handle}\") — refusing to update"
            )));
        }
    }
    Ok(patch)
}

async fn update_with_merge(
    client: &GrampsClient,
    endpoint: &str,
    handle: &str,
    data: &serde_json::Value,
) -> Result<serde_json::Value> {
    let patch = validate_patch(data, handle)?;

    let mut current: serde_json::Value = client.get(&format!("/api/{endpoint}/{handle}")).await?;
    let current_obj = current.as_object_mut().ok_or_else(|| {
        Error::Parse(format!(
            "expected GET /api/{endpoint}/{handle} to return a JSON object"
        ))
    })?;
    for (key, value) in patch {
        current_obj.insert(key.clone(), value.clone());
    }

    client
        .put(&format!("/api/{endpoint}/{handle}"), &current)
        .await
}

pub async fn update_person(
    client: &GrampsClient,
    handle: &str,
    data: &serde_json::Value,
) -> Result<serde_json::Value> {
    update_with_merge(client, "people", handle, data).await
}

pub async fn update_family(
    client: &GrampsClient,
    handle: &str,
    data: &serde_json::Value,
) -> Result<serde_json::Value> {
    update_with_merge(client, "families", handle, data).await
}

pub async fn update_event(
    client: &GrampsClient,
    handle: &str,
    data: &serde_json::Value,
) -> Result<serde_json::Value> {
    update_with_merge(client, "events", handle, data).await
}

pub async fn update_place(
    client: &GrampsClient,
    handle: &str,
    data: &serde_json::Value,
) -> Result<serde_json::Value> {
    update_with_merge(client, "places", handle, data).await
}

pub async fn update_source(
    client: &GrampsClient,
    handle: &str,
    data: &serde_json::Value,
) -> Result<serde_json::Value> {
    update_with_merge(client, "sources", handle, data).await
}

pub async fn update_note(
    client: &GrampsClient,
    handle: &str,
    data: &serde_json::Value,
) -> Result<serde_json::Value> {
    update_with_merge(client, "notes", handle, data).await
}

pub async fn update_citation(
    client: &GrampsClient,
    handle: &str,
    data: &serde_json::Value,
) -> Result<serde_json::Value> {
    update_with_merge(client, "citations", handle, data).await
}

pub async fn update_repository(
    client: &GrampsClient,
    handle: &str,
    data: &serde_json::Value,
) -> Result<serde_json::Value> {
    update_with_merge(client, "repositories", handle, data).await
}

pub async fn update_tag(
    client: &GrampsClient,
    handle: &str,
    data: &serde_json::Value,
) -> Result<serde_json::Value> {
    update_with_merge(client, "tags", handle, data).await
}

pub async fn update_media(
    client: &GrampsClient,
    handle: &str,
    data: &serde_json::Value,
) -> Result<serde_json::Value> {
    update_with_merge(client, "media", handle, data).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_object_data() {
        let err = validate_patch(&serde_json::json!("not an object"), "H1").unwrap_err();
        assert!(matches!(err, Error::Validation(_)), "got: {err:?}");
    }

    #[test]
    fn rejects_empty_data() {
        let err = validate_patch(&serde_json::json!({}), "H1").unwrap_err();
        assert!(matches!(err, Error::Validation(_)), "got: {err:?}");
    }

    #[test]
    fn rejects_mismatched_handle_in_data() {
        let err = validate_patch(&serde_json::json!({"handle": "H2"}), "H1").unwrap_err();
        assert!(matches!(err, Error::Validation(_)), "got: {err:?}");
    }

    #[test]
    fn accepts_matching_handle_in_data() {
        let data = serde_json::json!({"handle": "H1", "gender": 1});
        let patch = validate_patch(&data, "H1").unwrap();
        assert_eq!(patch["gender"], 1);
    }
}
