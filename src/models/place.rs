use super::Handle;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Place {
    pub handle: Handle,
    pub gramps_id: Option<String>,
    pub title: Option<String>,
    pub name: Option<PlaceName>,
    pub alt_names: Option<Vec<PlaceName>>,
    pub place_type: Option<serde_json::Value>,
    pub code: Option<String>,
    pub alt_loc: Option<Vec<serde_json::Value>>,
    pub urls: Option<Vec<serde_json::Value>>,
    pub placeref_list: Option<Vec<serde_json::Value>>,
    pub lat: Option<String>,
    pub long: Option<String>,
    pub media_list: Option<Vec<serde_json::Value>>,
    pub citation_list: Option<Vec<Handle>>,
    pub note_list: Option<Vec<Handle>>,
    pub change: Option<i64>,
    pub tag_list: Option<Vec<Handle>>,
    pub private: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlaceName {
    pub value: Option<String>,
    pub date: Option<serde_json::Value>,
    pub lang: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreatePlaceRequest {
    pub title: Option<String>,
    pub name: Option<PlaceName>,
    pub place_type: Option<serde_json::Value>,
    pub lat: Option<String>,
    pub long: Option<String>,
    pub note_list: Option<Vec<Handle>>,
}
