use serde::{Deserialize, Serialize};

use super::Handle;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Repository {
    pub handle: Option<Handle>,
    pub gramps_id: Option<String>,
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub repo_type: Option<String>,
    pub note_list: Option<Vec<Handle>>,
    pub address_list: Option<Vec<serde_json::Value>>,
    pub urls: Option<Vec<serde_json::Value>>,
    pub tag_list: Option<Vec<Handle>>,
    pub private: Option<bool>,
}
