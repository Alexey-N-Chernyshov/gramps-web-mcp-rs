use crate::client::{GrampsClient, Result};

pub async fn list_transactions(client: &GrampsClient) -> Result<serde_json::Value> {
    client.get("/api/transactions/").await
}

pub async fn undo_transaction(client: &GrampsClient, id: i64) -> Result<()> {
    client
        .post::<_, serde_json::Value>(
            &format!("/api/transactions/{id}/undo"),
            &serde_json::json!({}),
        )
        .await?;
    Ok(())
}
