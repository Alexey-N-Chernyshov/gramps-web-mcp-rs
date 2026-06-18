use super::Handle;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub handle: Handle,
    pub gramps_id: Option<String>,
    pub title: Option<String>,
    pub author: Option<String>,
    pub pubinfo: Option<String>,
    pub abbrev: Option<String>,
    pub media_list: Option<Vec<serde_json::Value>>,
    pub note_list: Option<Vec<Handle>>,
    pub change: Option<i64>,
    pub tag_list: Option<Vec<Handle>>,
    pub private: Option<bool>,
    pub reporef_list: Option<Vec<serde_json::Value>>,
    pub attribute_list: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreateSourceRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pubinfo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub abbrev: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note_list: Option<Vec<Handle>>,
}
