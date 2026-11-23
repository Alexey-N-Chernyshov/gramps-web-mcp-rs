// Copyright 2026 Alexey Chernyshov
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

pub mod event;
pub mod family;
pub mod person;
pub mod place;
pub mod source;

use serde::{Deserialize, Serialize};

/// Gramps object handle (opaque string ID).
pub type Handle = String;

/// A name-value date used in many Gramps objects.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GrampsDate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modifier: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dateval: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sortval: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub newyear: Option<i32>,
}
