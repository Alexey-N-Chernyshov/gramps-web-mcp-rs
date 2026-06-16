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
    pub title: Option<String>,
    pub author: Option<String>,
    pub pubinfo: Option<String>,
    pub abbrev: Option<String>,
    pub note_list: Option<Vec<Handle>>,
}
