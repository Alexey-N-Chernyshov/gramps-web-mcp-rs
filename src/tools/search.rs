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
    if let Some(p) = page {
        params.push(format!("page={p}"));
    }
    if let Some(ps) = pagesize {
        params.push(format!("pagesize={ps}"));
    }
    let path = format!("/api/search/?{}", params.join("&"));
    client.get(&path).await
}
