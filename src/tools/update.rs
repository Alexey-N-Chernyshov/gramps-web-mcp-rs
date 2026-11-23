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

/// The URL for update's own read-modify-write GET — deliberately never takes
/// `keys`/`strip`/`profile` (see `fetch_path_has_no_query_params` below): any such filter
/// would hand `current_obj` an incomplete object, and the PUT that follows replaces the
/// record wholesale, so whatever the GET left out is gone for good. `profile` specifically
/// is a read-only, server-computed summary — it must never round-trip into a PUT body.
fn fetch_path(endpoint: &str, handle: &str) -> String {
    format!("/api/{endpoint}/{handle}")
}

/// JSON Merge Patch (RFC 7396 §2 `MergePatch`): non-object `patch` replaces `target`
/// wholesale; object `patch` coerces `target` to an object and merges keys in recursively,
/// with `null` deleting rather than being copied in.
fn merge_patch(target: &mut serde_json::Value, patch: &serde_json::Value) {
    let Some(patch_obj) = patch.as_object() else {
        *target = patch.clone();
        return;
    };
    if target.as_object().is_none() {
        *target = serde_json::Value::Object(serde_json::Map::new());
    }
    let target_obj = target.as_object_mut().expect("just coerced to an object");
    for (key, value) in patch_obj {
        if value.is_null() {
            target_obj.remove(key);
        } else {
            merge_patch(
                target_obj
                    .entry(key.clone())
                    .or_insert(serde_json::Value::Null),
                value,
            );
        }
    }
}

/// Read-modify-write a JSON Merge Patch (RFC 7396) onto the object at
/// `/api/{endpoint}/{handle}`.
///
/// Gramps Web's PUT replaces the object wholesale, so sending just the fields the caller
/// wants to change would silently drop everything else (citation_list, note_list, ...).
/// This fetches the current object first and merges `data` onto it per RFC 7396 before
/// writing it back: nested objects merge recursively, while a list or scalar in `data`
/// replaces the corresponding value whole, and `null` removes a field.
///
/// There is an inherent GET/PUT race: a concurrent write between the two could be lost.
async fn update_with_merge(
    client: &GrampsClient,
    endpoint: &str,
    handle: &str,
    data: &serde_json::Value,
) -> Result<serde_json::Value> {
    let patch = validate_patch(data, handle)?;

    let mut current: serde_json::Value = client.get(&fetch_path(endpoint, handle)).await?;
    if current.as_object().is_none() {
        return Err(Error::Parse(format!(
            "expected GET /api/{endpoint}/{handle} to return a JSON object"
        )));
    }
    merge_patch(&mut current, &serde_json::Value::Object(patch.clone()));

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

    #[test]
    fn merge_patch_merges_nested_objects_recursively() {
        let mut target = serde_json::json!({"primary_name": {"first_name": "Ivan", "surname_list": ["Petrov"], "type": "Birth Name"}});
        let patch = serde_json::json!({"primary_name": {"first_name": "Petr"}});
        merge_patch(&mut target, &patch);
        assert_eq!(
            target,
            serde_json::json!({"primary_name": {"first_name": "Petr", "surname_list": ["Petrov"], "type": "Birth Name"}})
        );
    }

    #[test]
    fn merge_patch_replaces_arrays_wholesale() {
        let mut target = serde_json::json!({"tag_list": ["a", "b", "c"]});
        let patch = serde_json::json!({"tag_list": ["z"]});
        merge_patch(&mut target, &patch);
        assert_eq!(target, serde_json::json!({"tag_list": ["z"]}));
    }

    #[test]
    fn merge_patch_null_removes_key() {
        let mut target = serde_json::json!({"a": "b", "c": "d"});
        let patch = serde_json::json!({"a": null});
        merge_patch(&mut target, &patch);
        assert_eq!(target, serde_json::json!({"c": "d"}));
    }

    #[test]
    fn merge_patch_scalar_replaces_object() {
        let mut target = serde_json::json!({"a": {"b": "c"}});
        let patch = serde_json::json!({"a": "scalar"});
        merge_patch(&mut target, &patch);
        assert_eq!(target, serde_json::json!({"a": "scalar"}));
    }

    #[test]
    fn merge_patch_object_replaces_scalar() {
        let mut target = serde_json::json!({"a": "scalar"});
        let patch = serde_json::json!({"a": {"b": "c"}});
        merge_patch(&mut target, &patch);
        assert_eq!(target, serde_json::json!({"a": {"b": "c"}}));
    }

    /// The worked example from RFC 7396 §3.
    #[test]
    fn merge_patch_rfc7396_section_3_example() {
        let mut target = serde_json::json!({
            "a": "b",
            "c": {
                "d": "e",
                "f": "g"
            }
        });
        let patch = serde_json::json!({
            "a": "z",
            "c": {
                "f": null
            }
        });
        merge_patch(&mut target, &patch);
        assert_eq!(
            target,
            serde_json::json!({
                "a": "z",
                "c": {
                    "d": "e"
                }
            })
        );
    }

    /// All 15 examples from RFC 7396 Appendix A.
    #[test]
    fn merge_patch_rfc7396_appendix_a_examples() {
        use serde_json::json;
        let cases = [
            (json!({"a":"b"}), json!({"a":"c"}), json!({"a":"c"})),
            (json!({"a":"b"}), json!({"b":"c"}), json!({"a":"b","b":"c"})),
            (json!({"a":"b"}), json!({"a":null}), json!({})),
            (
                json!({"a":"b","b":"c"}),
                json!({"a":null}),
                json!({"b":"c"}),
            ),
            (json!({"a":["b"]}), json!({"a":"c"}), json!({"a":"c"})),
            (json!({"a":"c"}), json!({"a":["b"]}), json!({"a":["b"]})),
            (
                json!({"a":{"b":"c"}}),
                json!({"a":{"b":"d","c":null}}),
                json!({"a":{"b":"d"}}),
            ),
            (json!({"a":[{"b":"c"}]}), json!({"a":[1]}), json!({"a":[1]})),
            (json!(["a", "b"]), json!(["c", "d"]), json!(["c", "d"])),
            (json!({"a":"b"}), json!(["c"]), json!(["c"])),
            (json!({"a":"foo"}), json!(null), json!(null)),
            (json!({"a":"foo"}), json!("bar"), json!("bar")),
            (json!({"e":null}), json!({"a":1}), json!({"e":null,"a":1})),
            (json!([1, 2]), json!({"a":"b","c":null}), json!({"a":"b"})),
            (
                json!({}),
                json!({"a":{"bb":{"ccc":null}}}),
                json!({"a":{"bb":{}}}),
            ),
        ];
        for (original, patch, expected) in cases {
            let mut target = original.clone();
            merge_patch(&mut target, &patch);
            assert_eq!(target, expected, "original {original}, patch {patch}");
        }
    }

    #[test]
    fn fetch_path_has_no_query_params() {
        let path = fetch_path("people", "H1");
        assert_eq!(path, "/api/people/H1");
        assert!(
            !path.contains('?'),
            "update's own read-modify-write GET must fetch the full, unfiltered object \
             (no keys/strip/etc.) — filtering it would silently drop whatever it excluded \
             once the merged object is PUT back, got: {path}"
        );
    }
}
