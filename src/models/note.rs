use serde::{Deserialize, Serialize};

use super::Handle;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Note {
    pub handle: Option<Handle>,
    pub gramps_id: Option<String>,
    pub text: Option<serde_json::Value>,
    #[serde(rename = "type")]
    pub note_type: Option<String>,
    pub format: Option<i32>,
    pub tag_list: Option<Vec<Handle>>,
    pub private: Option<bool>,
}
