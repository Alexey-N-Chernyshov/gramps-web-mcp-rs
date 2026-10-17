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
/// (gramps_id/rules/page/pagesize/handles). `keys`/`strip`/`profile` today; expected growth
/// spot for anything else that's just "ask for more/less of the same object" — add a field
/// here and a branch in `query_params`. `handles` lives on `get_object_collection` itself,
/// not here, since (unlike these) it's mutually exclusive with `rules`/`page`/`pagesize`.
#[derive(Default, Clone, Copy)]
pub struct ReadOptions<'a> {
    pub keys: Option<&'a [String]>,
    /// `None` or `Some(true)` strips empty-valued fields (empty lists/strings, null dates)
    /// — that's our tools' default, the opposite of Gramps Web's own (`strip=false` unless
    /// asked). Pass `Some(false)` to get the complete object, empty fields included.
    pub strip: Option<bool>,
    pub profile: Option<&'a [String]>,
}

impl ReadOptions<'_> {
    /// Read options for internal plumbing (media/merge helpers, anything doing its own
    /// read-modify-write) that must always see the complete object: `strip`'s tools-level
    /// default would otherwise drop empty fields before a merge, the same risk `keys` and
    /// `profile` already have to be guarded against. Equivalent to `ReadOptions { strip:
    /// Some(false), ..Default::default() }` — a named constructor so the intent reads at
    /// the call site instead of as a bare `false`.
    pub fn full_object() -> ReadOptions<'static> {
        ReadOptions {
            strip: Some(false),
            ..ReadOptions::default()
        }
    }

    fn query_params(&self) -> Vec<String> {
        let mut params = Vec::new();
        if let Some(pp) = csv_param("profile", self.profile) {
            params.push(pp);
        }
        if let Some(kp) = csv_param("keys", self.effective_keys().as_deref()) {
            params.push(kp);
        }
        if self.strip != Some(false) {
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
    let collection = get_object_collection(
        client,
        endpoint,
        CollectionQuery {
            gramps_id: Some(gramps_id),
            ..Default::default()
        },
        opts,
    )
    .await?;
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

/// Maximum number of `handles` per `list_objects` call. Comma-joined handles land in the
/// request's query string (~28 chars each, including URL-encoding and the separator); a
/// reverse proxy's default header-size limit (nginx's ~8KB, say) caps how many fit in one
/// request, so this stays comfortably under that regardless of deployment.
pub const MAX_HANDLES: usize = 100;

/// Validates `list_objects`'s `handles` argument before any API call:
/// - Gramps Web ignores `rules`/`page`/`pagesize`/`sort` entirely once `handles` is given
///   (it's handled before filtering/sorting/pagination) — rejected here instead of silently
///   dropped.
/// - more than `MAX_HANDLES` handles is rejected rather than silently truncated or sent as
///   an oversized request that risks a proxy error.
///
/// A no-op when `handles` is `None` or empty.
pub fn validate_handles(
    handles: Option<&[String]>,
    rules: Option<&serde_json::Value>,
    page: Option<u32>,
    pagesize: Option<u32>,
    sort: Option<&[String]>,
) -> std::result::Result<(), String> {
    let Some(handles) = handles.filter(|h| !h.is_empty()) else {
        return Ok(());
    };
    if rules.is_some()
        || page.is_some()
        || pagesize.is_some()
        || sort.is_some_and(|s| !s.is_empty())
    {
        return Err(
            "`handles` cannot be combined with `rules`/`page`/`pagesize`/`sort` — Gramps Web \
             ignores them once `handles` is given (handles are resolved before filtering, \
             sorting, or pagination), so combining would silently drop them instead of \
             erroring. Make separate calls."
                .to_string(),
        );
    }
    if handles.len() > MAX_HANDLES {
        return Err(format!(
            "`handles` has {} entries, which exceeds the limit of {MAX_HANDLES} — split into \
             batches of ≤{MAX_HANDLES}.",
            handles.len()
        ));
    }
    Ok(())
}

/// `sort` keys common to every object type (gramps-web-api 3.23.1, `api/resources/sort.py`).
const COMMON_SORT_KEYS: &[&str] = &["gramps_id", "change", "private"];

/// `sort` keys specific to each object type, on top of `COMMON_SORT_KEYS`. Endpoints not
/// listed here (notes, citations, repositories) only support the common keys.
fn type_specific_sort_keys(endpoint: &str) -> &'static [&'static str] {
    match endpoint {
        "people" => &["surname", "name", "soundex", "birth", "death", "gender"],
        "families" => &["surname", "type", "soundex"],
        "events" => &["date", "type", "description", "place"],
        "places" => &["title", "type", "latitude", "longitude"],
        "sources" => &["title", "author", "pubinfo", "abbrev"],
        "media" => &["title", "path", "mime", "date"],
        "tags" => &["name", "color", "priority"],
        _ => &[],
    }
}

/// Validates `list_objects`'s `sort` keys against the allowed set for `endpoint` before any
/// API call — an unknown key otherwise gets a bare 422 from Gramps Web with no indication of
/// what's actually valid, which an LLM can't self-correct from. A no-op when `sort` is
/// `None` or empty.
pub fn validate_sort(endpoint: &str, sort: Option<&[String]>) -> std::result::Result<(), String> {
    let Some(sort) = sort.filter(|s| !s.is_empty()) else {
        return Ok(());
    };
    let mut allowed = COMMON_SORT_KEYS.to_vec();
    allowed.extend(type_specific_sort_keys(endpoint));
    for key in sort {
        let bare = key.strip_prefix('-').unwrap_or(key);
        if !allowed.contains(&bare) {
            return Err(format!(
                "unknown sort key \"{key}\" for \"{endpoint}\" — allowed: {}",
                allowed.join(", ")
            ));
        }
    }
    Ok(())
}

/// Structural parameters selecting *which* objects `list_objects` returns from a collection,
/// and in what order — as opposed to `ReadOptions`, which only affects how much of each
/// selected object comes back. `handles` is mutually exclusive with the rest in practice
/// (Gramps Web silently ignores `rules`/`page`/`pagesize`/`sort` once `handles` is given);
/// see `validate_handles`/`validate_sort`.
#[derive(Default)]
pub struct CollectionQuery<'a> {
    pub gramps_id: Option<&'a str>,
    pub rules: Option<&'a serde_json::Value>,
    pub page: Option<u32>,
    pub pagesize: Option<u32>,
    pub handles: Option<&'a [String]>,
    pub sort: Option<&'a [String]>,
}

pub async fn get_object_collection(
    client: &GrampsClient,
    endpoint: &str,
    query: CollectionQuery<'_>,
    opts: ReadOptions<'_>,
) -> Result<serde_json::Value> {
    let mut params: Vec<String> = Vec::new();
    if let Some(id) = query.gramps_id {
        params.push(format!("gramps_id={}", urlencoding::encode(id)));
    }
    if let Some(r) = query.rules {
        let encoded = serde_json::to_string(r).unwrap_or_default();
        params.push(format!("rules={}", urlencoding::encode(&encoded)));
    }
    // Gramps Web defaults `page` to 0, which disables pagination entirely and
    // makes it silently ignore `pagesize`. If the caller wants a capped page but
    // didn't say which one, assume the first.
    let page = query.page.or(query.pagesize.is_some().then_some(1));
    if let Some(p) = page {
        params.push(format!("page={p}"));
    }
    if let Some(ps) = query.pagesize {
        params.push(format!("pagesize={ps}"));
    }
    if let Some(hp) = csv_param("handles", query.handles) {
        params.push(hp);
    }
    if let Some(sp) = csv_param("sort", query.sort) {
        params.push(sp);
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
    fn validate_handles_none_or_empty_is_always_ok() {
        assert!(validate_handles(None, None, None, None, None).is_ok());
        let empty: Vec<String> = vec![];
        assert!(validate_handles(
            Some(&empty),
            Some(&serde_json::json!({})),
            Some(1),
            Some(1),
            Some(&empty)
        )
        .is_ok());
    }

    #[test]
    fn validate_handles_alone_is_ok() {
        let handles = vec!["H1".to_string(), "H2".to_string()];
        assert!(validate_handles(Some(&handles), None, None, None, None).is_ok());
    }

    #[test]
    fn validate_handles_rejects_rules() {
        let handles = vec!["H1".to_string()];
        let rules = serde_json::json!({"rules": []});
        let err = validate_handles(Some(&handles), Some(&rules), None, None, None).unwrap_err();
        assert!(err.contains("rules"), "got: {err}");
    }

    #[test]
    fn validate_handles_rejects_page_or_pagesize() {
        let handles = vec!["H1".to_string()];
        assert!(validate_handles(Some(&handles), None, Some(1), None, None).is_err());
        assert!(validate_handles(Some(&handles), None, None, Some(20), None).is_err());
    }

    #[test]
    fn validate_handles_rejects_sort() {
        let handles = vec!["H1".to_string()];
        let sort = vec!["birth".to_string()];
        let err = validate_handles(Some(&handles), None, None, None, Some(&sort)).unwrap_err();
        assert!(err.contains("sort"), "got: {err}");
    }

    #[test]
    fn validate_handles_rejects_more_than_max() {
        let handles: Vec<String> = (0..=MAX_HANDLES).map(|i| format!("H{i}")).collect();
        let err = validate_handles(Some(&handles), None, None, None, None).unwrap_err();
        assert!(err.contains(&MAX_HANDLES.to_string()), "got: {err}");
    }

    #[test]
    fn validate_handles_accepts_exactly_max() {
        let handles: Vec<String> = (0..MAX_HANDLES).map(|i| format!("H{i}")).collect();
        assert!(validate_handles(Some(&handles), None, None, None, None).is_ok());
    }

    #[test]
    fn validate_sort_none_or_empty_is_always_ok() {
        assert!(validate_sort("people", None).is_ok());
        let empty: Vec<String> = vec![];
        assert!(validate_sort("people", Some(&empty)).is_ok());
    }

    #[test]
    fn validate_sort_accepts_common_keys_for_any_type() {
        let sort = vec!["gramps_id".to_string(), "-change".to_string()];
        assert!(validate_sort("notes", Some(&sort)).is_ok());
    }

    #[test]
    fn validate_sort_accepts_type_specific_keys() {
        let sort = vec!["birth".to_string()];
        assert!(validate_sort("people", Some(&sort)).is_ok());
        let sort = vec!["-birth".to_string()];
        assert!(
            validate_sort("people", Some(&sort)).is_ok(),
            "descending prefix should strip before matching"
        );
    }

    #[test]
    fn validate_sort_rejects_key_from_a_different_type() {
        // "birth" is person-specific, not valid for events.
        let sort = vec!["birth".to_string()];
        let err = validate_sort("events", Some(&sort)).unwrap_err();
        assert!(err.contains("birth"), "got: {err}");
        assert!(
            err.contains("date"),
            "should list events' allowed keys, got: {err}"
        );
    }

    #[test]
    fn validate_sort_rejects_unknown_key() {
        let sort = vec!["not_a_real_key".to_string()];
        let err = validate_sort("people", Some(&sort)).unwrap_err();
        assert!(err.contains("not_a_real_key"), "got: {err}");
        assert!(
            err.contains("gramps_id"),
            "should list allowed keys, got: {err}"
        );
    }

    #[test]
    fn validate_sort_endpoint_without_specific_keys_still_allows_common_keys() {
        let sort = vec!["private".to_string()];
        assert!(validate_sort("citations", Some(&sort)).is_ok());
        let sort = vec!["title".to_string()];
        assert!(
            validate_sort("citations", Some(&sort)).is_err(),
            "\"title\" is not a valid sort key for citations"
        );
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
    fn read_options_default_strips_by_default() {
        // No `strip` given at all (e.g. a caller who never heard of it) still gets the
        // tools' default: strip empty fields.
        assert_eq!(
            ReadOptions::default().query_params(),
            vec!["strip=true".to_string()],
            "ReadOptions::default() must strip by default, matching get_object/list_objects's own default"
        );
    }

    #[test]
    fn read_options_strip_none_or_true_adds_strip_param() {
        for strip in [None, Some(true)] {
            let opts = ReadOptions {
                strip,
                ..Default::default()
            };
            assert_eq!(
                opts.query_params(),
                vec!["strip=true".to_string()],
                "strip={strip:?} should add strip=true"
            );
        }
    }

    #[test]
    fn read_options_strip_false_omits_strip_param() {
        let opts = ReadOptions {
            strip: Some(false),
            ..Default::default()
        };
        assert_eq!(
            opts.query_params(),
            Vec::<String>::new(),
            "strip=false should opt back into the full object (Gramps Web's own default)"
        );
    }

    #[test]
    fn read_options_combines_keys_and_strip() {
        let keys = vec!["gramps_id".to_string()];
        let opts = ReadOptions {
            keys: Some(&keys),
            ..Default::default()
        };
        assert_eq!(
            opts.query_params(),
            vec!["keys=gramps_id".to_string(), "strip=true".to_string()]
        );
    }

    #[test]
    fn full_object_disables_strip_and_adds_no_other_params() {
        assert_eq!(
            ReadOptions::full_object().query_params(),
            Vec::<String>::new(),
            "internal read-modify-write plumbing must fetch the complete, unfiltered \
             object — same contract as update's own fetch_path"
        );
    }

    #[test]
    fn read_options_profile_adds_profile_param() {
        let profile = vec!["self".to_string()];
        let opts = ReadOptions {
            profile: Some(&profile),
            strip: Some(false),
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
            strip: Some(false),
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
            strip: Some(false),
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
