use super::{GrampsDate, Handle};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub handle: Handle,
    pub gramps_id: Option<String>,
    #[serde(rename = "type")]
    pub event_type: Option<serde_json::Value>,
    pub date: Option<GrampsDate>,
    pub place: Option<Handle>,
    pub description: Option<String>,
    pub media_list: Option<Vec<serde_json::Value>>,
    pub citation_list: Option<Vec<Handle>>,
    pub note_list: Option<Vec<Handle>>,
    pub change: Option<i64>,
    pub tag_list: Option<Vec<Handle>>,
    pub private: Option<bool>,
    pub attribute_list: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreateEventRequest {
    #[serde(rename = "type")]
    pub event_type: Option<serde_json::Value>,
    pub date: Option<GrampsDate>,
    pub place: Option<Handle>,
    pub description: Option<String>,
    pub note_list: Option<Vec<Handle>>,
    pub citation_list: Option<Vec<Handle>>,
}
