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
use urlencoding;

/// Returns the live catalog of native Gramps filter rules (and saved custom filters) for
/// one object type (`GET /api/filters/{endpoint}/`): name, description, labels and
/// parameter types for every rule usable in `list_objects`'s `rules` field, including
/// `MatchesQuery` (a GOQL expression). Tag has no rule catalog of its own upstream.
pub async fn get_filter_rules(client: &GrampsClient, endpoint: &str) -> Result<serde_json::Value> {
    client.get(&format!("/api/filters/{endpoint}")).await
}

pub async fn get_relations(
    client: &GrampsClient,
    handle1: &str,
    handle2: &str,
) -> Result<serde_json::Value> {
    client
        .get(&format!("/api/relations/{handle1}/{handle2}"))
        .await
}

pub async fn get_person_timeline(client: &GrampsClient, handle: &str) -> Result<serde_json::Value> {
    client.get(&format!("/api/people/{handle}/timeline")).await
}

pub async fn get_family_timeline(client: &GrampsClient, handle: &str) -> Result<serde_json::Value> {
    client
        .get(&format!("/api/families/{handle}/timeline"))
        .await
}

pub async fn get_event_span(
    client: &GrampsClient,
    handle1: &str,
    handle2: &str,
) -> Result<serde_json::Value> {
    client
        .get(&format!("/api/events/{handle1}/span/{handle2}"))
        .await
}

/// Returns tree-level statistics (person count, family count, etc.).
pub async fn get_tree_info(client: &GrampsClient) -> Result<serde_json::Value> {
    client.get("/api/metadata/").await
}

/// How `get_object` was asked to identify its target — exactly one of `handle`/`gramps_id`.
#[derive(Debug)]
pub enum ObjectLookup {
    Handle(String),
    GrampsId(String),
}

/// Validates `get_object`'s `handle`/`gramps_id` arguments: exactly one must be given. The
/// `Err` string is the user-facing tool error message.
pub fn resolve_object_lookup(
    handle: Option<String>,
    gramps_id: Option<String>,
) -> std::result::Result<ObjectLookup, &'static str> {
    match (handle, gramps_id) {
        (Some(h), None) => Ok(ObjectLookup::Handle(h)),
        (None, Some(id)) => Ok(ObjectLookup::GrampsId(id)),
        (Some(_), Some(_)) => Err("Provide only one of `handle` or `gramps_id`, not both."),
        (None, None) => Err(
            "Provide exactly one of `handle` or `gramps_id` to identify the object. \
             To browse or filter a collection instead, use `list_objects`.",
        ),
    }
}

pub async fn get_object(
    client: &GrampsClient,
    endpoint: &str,
    lookup: ObjectLookup,
) -> Result<serde_json::Value> {
    match lookup {
        ObjectLookup::Handle(h) => get_object_by_handle(client, endpoint, &h).await,
        ObjectLookup::GrampsId(id) => get_object_by_gramps_id(client, endpoint, &id).await,
    }
}

pub async fn get_object_by_handle(
    client: &GrampsClient,
    endpoint: &str,
    handle: &str,
) -> Result<serde_json::Value> {
    client.get(&format!("/api/{endpoint}/{handle}")).await
}

/// Resolves a Gramps ID to a single object by filtering the collection endpoint. Gramps IDs
/// are not guaranteed unique, so this errors on anything but exactly one match rather than
/// silently returning one of several candidates.
pub async fn get_object_by_gramps_id(
    client: &GrampsClient,
    endpoint: &str,
    gramps_id: &str,
) -> Result<serde_json::Value> {
    let collection =
        get_object_collection(client, endpoint, Some(gramps_id), None, None, None).await?;
    let items = collection.as_array().cloned().unwrap_or_default();
    resolve_gramps_id_match(endpoint, gramps_id, items)
}

/// Picks the single match for a gramps_id out of the collection results, or produces a
/// descriptive error for zero or multiple matches. Split out from `get_object_by_gramps_id`
/// so the 0/1/many branching is unit-testable without a live backend.
fn resolve_gramps_id_match(
    endpoint: &str,
    gramps_id: &str,
    mut items: Vec<serde_json::Value>,
) -> Result<serde_json::Value> {
    match items.len() {
        0 => Err(Error::NotFound(format!(
            "no {endpoint} object with gramps_id \"{gramps_id}\""
        ))),
        1 => Ok(items.remove(0)),
        n => {
            let handles: Vec<&str> = items.iter().filter_map(|i| i["handle"].as_str()).collect();
            Err(Error::Validation(format!(
                "gramps_id \"{gramps_id}\" matches {n} {endpoint} objects (Gramps ID is not \
                 guaranteed unique) — handles: [{}]. Use `handle` to pick one.",
                handles.join(", ")
            )))
        }
    }
}

pub async fn get_object_collection(
    client: &GrampsClient,
    endpoint: &str,
    gramps_id: Option<&str>,
    rules: Option<&serde_json::Value>,
    page: Option<u32>,
    pagesize: Option<u32>,
) -> Result<serde_json::Value> {
    let mut params: Vec<String> = Vec::new();
    if let Some(id) = gramps_id {
        params.push(format!("gramps_id={}", urlencoding::encode(id)));
    }
    if let Some(r) = rules {
        let encoded = serde_json::to_string(r).unwrap_or_default();
        params.push(format!("rules={}", urlencoding::encode(&encoded)));
    }
    // Gramps Web defaults `page` to 0, which disables pagination entirely and
    // makes it silently ignore `pagesize`. If the caller wants a capped page but
    // didn't say which one, assume the first.
    let page = page.or(pagesize.is_some().then_some(1));
    if let Some(p) = page {
        params.push(format!("page={p}"));
    }
    if let Some(ps) = pagesize {
        params.push(format!("pagesize={ps}"));
    }
    let path = if params.is_empty() {
        format!("/api/{endpoint}/")
    } else {
        format!("/api/{endpoint}/?{}", params.join("&"))
    };
    client.get(&path).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_object_lookup_accepts_handle_only() {
        let lookup = resolve_object_lookup(Some("H1".to_string()), None).unwrap();
        assert!(matches!(lookup, ObjectLookup::Handle(h) if h == "H1"));
    }

    #[test]
    fn resolve_object_lookup_accepts_gramps_id_only() {
        let lookup = resolve_object_lookup(None, Some("I0001".to_string())).unwrap();
        assert!(matches!(lookup, ObjectLookup::GrampsId(id) if id == "I0001"));
    }

    #[test]
    fn resolve_object_lookup_rejects_both() {
        let err =
            resolve_object_lookup(Some("H1".to_string()), Some("I0001".to_string())).unwrap_err();
        assert!(err.contains("not both"), "got: {err}");
    }

    #[test]
    fn resolve_object_lookup_rejects_neither() {
        let err = resolve_object_lookup(None, None).unwrap_err();
        assert!(err.contains("exactly one"), "got: {err}");
    }

    #[test]
    fn resolve_gramps_id_match_zero_items_is_not_found() {
        let err = resolve_gramps_id_match("people", "I9999", vec![]).unwrap_err();
        assert!(matches!(err, Error::NotFound(_)), "got: {err:?}");
    }

    #[test]
    fn resolve_gramps_id_match_one_item_returns_the_object_not_an_array() {
        let item = serde_json::json!({"handle": "H1", "gramps_id": "I0001"});
        let result = resolve_gramps_id_match("people", "I0001", vec![item.clone()]).unwrap();
        assert!(result.is_object(), "expected a single object, got {result}");
        assert_eq!(result, item);
    }

    #[test]
    fn resolve_gramps_id_match_multiple_items_lists_their_handles() {
        let items = vec![
            serde_json::json!({"handle": "H1", "gramps_id": "I0001"}),
            serde_json::json!({"handle": "H2", "gramps_id": "I0001"}),
        ];
        let err = resolve_gramps_id_match("people", "I0001", items).unwrap_err();
        assert!(matches!(err, Error::Validation(_)), "got: {err:?}");
        let msg = err.to_string();
        assert!(msg.contains("H1"), "error should list handle H1: {msg}");
        assert!(msg.contains("H2"), "error should list handle H2: {msg}");
    }
}
