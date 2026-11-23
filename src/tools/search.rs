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

pub async fn search(
    client: &GrampsClient,
    query: &str,
    object_type: Option<&str>,
    page: Option<u32>,
    pagesize: Option<u32>,
) -> Result<serde_json::Value> {
    let mut params = vec![format!("query={}", urlencoding::encode(query))];
    if let Some(t) = object_type {
        params.push(format!("type={}", urlencoding::encode(t)));
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
    let path = format!("/api/search/?{}", params.join("&"));
    client.get(&path).await
}
