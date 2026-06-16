use serde::{Deserialize, Serialize};

use super::Handle;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Citation {
    pub handle: Option<Handle>,
    pub gramps_id: Option<String>,
    pub source_handle: Option<String>,
    pub page: Option<String>,
    pub confidence: Option<i32>,
    pub date: Option<serde_json::Value>,
    pub note_list: Option<Vec<Handle>>,
    pub media_list: Option<Vec<serde_json::Value>>,
    pub tag_list: Option<Vec<Handle>>,
    pub private: Option<bool>,
}
