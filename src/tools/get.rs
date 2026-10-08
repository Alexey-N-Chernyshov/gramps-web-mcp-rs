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

use crate::client::{GrampsClient, Result};
use urlencoding;

/// Returns the live catalog of native Gramps filter rules (and saved custom filters) for
/// one object type (`GET /api/filters/{endpoint}/`): name, description, labels and
/// parameter types for every rule usable in `get_object`'s `rules` field, including
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

pub async fn get_object_by_handle(
    client: &GrampsClient,
    endpoint: &str,
    handle: &str,
) -> Result<serde_json::Value> {
    client.get(&format!("/api/{endpoint}/{handle}")).await
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
