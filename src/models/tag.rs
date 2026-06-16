use serde::{Deserialize, Serialize};

use super::Handle;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Tag {
    pub handle: Option<Handle>,
    pub name: Option<String>,
    pub color: Option<String>,
    pub priority: Option<i32>,
}
