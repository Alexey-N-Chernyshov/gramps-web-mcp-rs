use super::Handle;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Family {
    pub handle: Handle,
    pub gramps_id: Option<String>,
    pub father_handle: Option<Handle>,
    pub mother_handle: Option<Handle>,
    pub child_ref_list: Option<Vec<ChildRef>>,
    pub family_rel_type: Option<serde_json::Value>,
    pub event_ref_list: Option<Vec<serde_json::Value>>,
    pub media_list: Option<Vec<serde_json::Value>>,
    pub attribute_list: Option<Vec<serde_json::Value>>,
    pub lds_ord_list: Option<Vec<serde_json::Value>>,
    pub citation_list: Option<Vec<Handle>>,
    pub note_list: Option<Vec<Handle>>,
    pub change: Option<i64>,
    pub tag_list: Option<Vec<Handle>>,
    pub private: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildRef {
    #[serde(rename = "ref")]
    pub ref_handle: Option<Handle>,
    pub frel: Option<serde_json::Value>,
    pub mrel: Option<serde_json::Value>,
    pub private: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreateFamilyRequest {
    pub father_handle: Option<Handle>,
    pub mother_handle: Option<Handle>,
    pub child_ref_list: Option<Vec<serde_json::Value>>,
    pub family_rel_type: Option<serde_json::Value>,
    pub note_list: Option<Vec<Handle>>,
}
