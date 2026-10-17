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

/// Validates `get_object`'s `handle`/`gramps_id` arguments: exactly one must be given, and
/// `gramps_id` is rejected outright for `endpoint == "tags"` (tags have no Gramps ID) without
/// making an API call. The `Err` string is the user-facing tool error message.
pub fn resolve_object_lookup(
    endpoint: &str,
    handle: Option<String>,
    gramps_id: Option<String>,
) -> std::result::Result<ObjectLookup, &'static str> {
    match (handle, gramps_id) {
        (Some(h), None) => Ok(ObjectLookup::Handle(h)),
        (None, Some(_)) if endpoint == "tags" => Err("tags have no gramps_id — use handle"),
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
    opts: ReadOptions<'_>,
) -> Result<serde_json::Value> {
    match lookup {
        ObjectLookup::Handle(h) => get_object_by_handle(client, endpoint, &h, opts).await,
        ObjectLookup::GrampsId(id) => get_object_by_gramps_id(client, endpoint, &id, opts).await,
    }
}

/// Extra query options shared by every read path (`get_object`'s handle/gramps_id lookups
/// and `list_objects`'s collection browse) on top of their own structural parameters
/// (gramps_id/rules/page/pagesize). `keys`/`strip`/`profile` today; expected growth spot for
/// `handles`/`sort` — add a field here and a branch in `query_params`, not a new parameter
/// on every function below.
#[derive(Default, Clone, Copy)]
pub struct ReadOptions<'a> {
    pub keys: Option<&'a [String]>,
    pub strip: Option<bool>,
    pub profile: Option<&'a [String]>,
}

impl ReadOptions<'_> {
    fn query_params(&self) -> Vec<String> {
        let mut params = Vec::new();
        if let Some(pp) = csv_param("profile", self.profile) {
            params.push(pp);
        }
        if let Some(kp) = csv_param("keys", self.effective_keys().as_deref()) {
            params.push(kp);
        }
        if self.strip == Some(true) {
            params.push("strip=true".to_string());
        }
        params
    }

    /// Gramps Web applies `keys` *after* adding `profile` to the object, so a `keys` list
    /// that omits `"profile"` silently strips the profile summary right back out. If both
    /// are set and `keys` doesn't already mention `"profile"`, add it — the request asked
    /// for a profile, so it should actually come back.
    fn effective_keys(&self) -> Option<std::borrow::Cow<'_, [String]>> {
        let keys = self.keys.filter(|k| !k.is_empty())?;
        let wants_profile = self.profile.is_some_and(|p| !p.is_empty());
        if wants_profile && !keys.iter().any(|k| k == "profile") {
            let mut with_profile = keys.to_vec();
            with_profile.push("profile".to_string());
            Some(std::borrow::Cow::Owned(with_profile))
        } else {
            Some(std::borrow::Cow::Borrowed(keys))
        }
    }
}

/// Builds a `{name}=a,b,c` query parameter (each element URL-encoded individually, joined
/// with a literal comma) — the Gramps Web convention for list-valued query parameters like
/// `keys` and `profile`. `None`, or an empty list (nothing to ask for), omits the parameter
/// entirely rather than sending `{name}=`.
fn csv_param(name: &str, values: Option<&[String]>) -> Option<String> {
    let values = values.filter(|v| !v.is_empty())?;
    Some(format!(
        "{name}={}",
        values
            .iter()
            .map(|v| urlencoding::encode(v))
            .collect::<Vec<_>>()
            .join(",")
    ))
}

pub async fn get_object_by_handle(
    client: &GrampsClient,
    endpoint: &str,
    handle: &str,
    opts: ReadOptions<'_>,
) -> Result<serde_json::Value> {
    let params = opts.query_params();
    let path = if params.is_empty() {
        format!("/api/{endpoint}/{handle}")
    } else {
        format!("/api/{endpoint}/{handle}?{}", params.join("&"))
    };
    client.get(&path).await
}

/// Resolves a Gramps ID to a single object by filtering the collection endpoint. Gramps Web
/// itself only ever returns zero or one match for `?gramps_id=...` (404, or a one-element
/// array) — it never returns several, even if duplicate Gramps IDs exist in the tree.
pub async fn get_object_by_gramps_id(
    client: &GrampsClient,
    endpoint: &str,
    gramps_id: &str,
    opts: ReadOptions<'_>,
) -> Result<serde_json::Value> {
    let collection =
        get_object_collection(client, endpoint, Some(gramps_id), None, None, None, opts).await?;
    let items = expect_array(endpoint, collection)?;
    resolve_gramps_id_match(endpoint, gramps_id, items)
}

/// Takes a `?gramps_id=...` collection response apart by value, failing with a clear
/// `Error::Parse` (rather than `Error::NotFound`) if the backend didn't return a JSON array.
fn expect_array(endpoint: &str, collection: serde_json::Value) -> Result<Vec<serde_json::Value>> {
    match collection {
        serde_json::Value::Array(items) => Ok(items),
        other => Err(Error::Parse(format!(
            "expected GET /api/{endpoint}/?gramps_id=... to return a JSON array, got: {other}"
        ))),
    }
}

/// Picks the single match for a gramps_id out of the collection results, or produces a
/// descriptive error for zero matches. The multiple-matches branch is a defensive fallback,
/// not documented upstream behavior — Gramps Web's own `?gramps_id=...` filtering never
/// returns more than one match — kept in case that ever changes.
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
    opts: ReadOptions<'_>,
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
    params.extend(opts.query_params());
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
        let lookup = resolve_object_lookup("people", Some("H1".to_string()), None).unwrap();
        assert!(matches!(lookup, ObjectLookup::Handle(h) if h == "H1"));
    }

    #[test]
    fn resolve_object_lookup_accepts_gramps_id_only() {
        let lookup = resolve_object_lookup("people", None, Some("I0001".to_string())).unwrap();
        assert!(matches!(lookup, ObjectLookup::GrampsId(id) if id == "I0001"));
    }

    #[test]
    fn resolve_object_lookup_rejects_both() {
        let err =
            resolve_object_lookup("people", Some("H1".to_string()), Some("I0001".to_string()))
                .unwrap_err();
        assert!(err.contains("not both"), "got: {err}");
    }

    #[test]
    fn resolve_object_lookup_rejects_neither() {
        let err = resolve_object_lookup("people", None, None).unwrap_err();
        assert!(err.contains("exactly one"), "got: {err}");
    }

    #[test]
    fn resolve_object_lookup_rejects_gramps_id_for_tags() {
        let err = resolve_object_lookup("tags", None, Some("I0001".to_string())).unwrap_err();
        assert!(err.contains("gramps_id"), "got: {err}");
        assert!(err.contains("handle"), "got: {err}");
    }

    #[test]
    fn resolve_object_lookup_allows_handle_for_tags() {
        let lookup = resolve_object_lookup("tags", Some("T1".to_string()), None).unwrap();
        assert!(matches!(lookup, ObjectLookup::Handle(h) if h == "T1"));
    }

    #[test]
    fn csv_param_none_omits_the_parameter() {
        assert_eq!(csv_param("keys", None), None);
    }

    #[test]
    fn csv_param_empty_slice_omits_the_parameter() {
        assert_eq!(csv_param("keys", Some(&[])), None);
    }

    #[test]
    fn csv_param_joins_and_encodes_each_element() {
        let keys = vec!["gramps_id".to_string(), "primary_name".to_string()];
        assert_eq!(
            csv_param("keys", Some(&keys)),
            Some("keys=gramps_id,primary_name".to_string())
        );
    }

    #[test]
    fn csv_param_url_encodes_special_characters_per_element() {
        let keys = vec!["a b".to_string(), "c".to_string()];
        assert_eq!(
            csv_param("keys", Some(&keys)),
            Some("keys=a%20b,c".to_string())
        );
    }

    #[test]
    fn csv_param_uses_the_given_name() {
        let values = vec!["self".to_string()];
        assert_eq!(
            csv_param("profile", Some(&values)),
            Some("profile=self".to_string())
        );
    }

    #[test]
    fn read_options_default_has_no_params() {
        assert_eq!(ReadOptions::default().query_params(), Vec::<String>::new());
    }

    #[test]
    fn read_options_strip_true_adds_strip_param() {
        let opts = ReadOptions {
            strip: Some(true),
            ..Default::default()
        };
        assert_eq!(opts.query_params(), vec!["strip=true".to_string()]);
    }

    #[test]
    fn read_options_strip_false_or_none_omits_strip_param() {
        for strip in [Some(false), None] {
            let opts = ReadOptions {
                strip,
                ..Default::default()
            };
            assert_eq!(
                opts.query_params(),
                Vec::<String>::new(),
                "strip={strip:?} should not add a query parameter"
            );
        }
    }

    #[test]
    fn read_options_combines_keys_and_strip() {
        let keys = vec!["gramps_id".to_string()];
        let opts = ReadOptions {
            keys: Some(&keys),
            strip: Some(true),
            ..Default::default()
        };
        assert_eq!(
            opts.query_params(),
            vec!["keys=gramps_id".to_string(), "strip=true".to_string()]
        );
    }

    #[test]
    fn read_options_profile_adds_profile_param() {
        let profile = vec!["self".to_string()];
        let opts = ReadOptions {
            profile: Some(&profile),
            ..Default::default()
        };
        assert_eq!(opts.query_params(), vec!["profile=self".to_string()]);
    }

    #[test]
    fn read_options_profile_without_keys_does_not_add_keys_param() {
        let profile = vec!["self".to_string()];
        let opts = ReadOptions {
            profile: Some(&profile),
            ..Default::default()
        };
        assert!(
            !opts.query_params().iter().any(|p| p.starts_with("keys=")),
            "no `keys` was requested, so none should be added just because `profile` was"
        );
    }

    #[test]
    fn read_options_profile_plus_keys_without_profile_adds_it_to_keys() {
        let profile = vec!["self".to_string()];
        let keys = vec!["gramps_id".to_string()];
        let opts = ReadOptions {
            keys: Some(&keys),
            profile: Some(&profile),
            ..Default::default()
        };
        assert_eq!(
            opts.query_params(),
            vec![
                "profile=self".to_string(),
                "keys=gramps_id,profile".to_string(),
            ],
            "keys omitted \"profile\", so Gramps Web would strip the profile summary \
             right back out unless it's added automatically"
        );
    }

    #[test]
    fn read_options_profile_plus_keys_already_containing_profile_is_not_duplicated() {
        let profile = vec!["self".to_string()];
        let keys = vec!["gramps_id".to_string(), "profile".to_string()];
        let opts = ReadOptions {
            keys: Some(&keys),
            profile: Some(&profile),
            ..Default::default()
        };
        assert_eq!(
            opts.query_params(),
            vec![
                "profile=self".to_string(),
                "keys=gramps_id,profile".to_string(),
            ]
        );
    }

    #[test]
    fn expect_array_passes_through_an_array() {
        let items = expect_array("people", serde_json::json!([{"handle": "H1"}])).unwrap();
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn expect_array_rejects_non_array_with_parse_error() {
        let err = expect_array("people", serde_json::json!({"handle": "H1"})).unwrap_err();
        assert!(matches!(err, Error::Parse(_)), "got: {err:?}");
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
