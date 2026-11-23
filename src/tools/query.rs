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

/// `order_by` is a list of pre-built `{"column": ..., "direction": "asc"|"desc"}` objects —
/// the caller (the `query_object` tool handler) is responsible for that shaping, same as
/// every other tool in this module deals in plain JSON/primitives rather than schema types.
#[allow(clippy::too_many_arguments)]
pub async fn query_object(
    client: &GrampsClient,
    endpoint: &str,
    select: Option<Vec<String>>,
    where_expr: Option<String>,
    order_by: Option<Vec<serde_json::Value>>,
    limit: Option<u32>,
    after: Option<String>,
    count: Option<bool>,
) -> Result<serde_json::Value> {
    let mut body = serde_json::Map::new();
    if let Some(s) = select {
        body.insert("select".into(), serde_json::json!(s));
    }
    if let Some(w) = where_expr {
        body.insert("where_expr".into(), serde_json::json!(w));
    }
    if let Some(o) = order_by {
        body.insert("order_by".into(), serde_json::json!(o));
    }
    if let Some(l) = limit {
        body.insert("limit".into(), serde_json::json!(l));
    }
    if let Some(a) = after {
        body.insert("after".into(), serde_json::json!(a));
    }
    if count.unwrap_or(false) {
        body.insert("count".into(), serde_json::json!(true));
    }
    client
        .post_with_count(
            &format!("/api/{endpoint}/query/"),
            &serde_json::Value::Object(body),
        )
        .await
}
