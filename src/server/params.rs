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

use schemars::JsonSchema;
use serde::Deserialize;

#[derive(JsonSchema, Clone, Copy, Debug, strum::IntoStaticStr)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum ObjectType {
    Person,
    Family,
    Event,
    Place,
    Note,
    Citation,
    Source,
    Media,
    Repository,
    Tag,
}

const VALID_OBJECT_TYPES: &str =
    "person, family, event, place, note, citation, source, media, repository, tag";

impl<'de> serde::Deserialize<'de> for ObjectType {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = ObjectType;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "one of: {VALID_OBJECT_TYPES}")
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<ObjectType, E> {
                match v {
                    "person" => Ok(ObjectType::Person),
                    "family" => Ok(ObjectType::Family),
                    "event" => Ok(ObjectType::Event),
                    "place" => Ok(ObjectType::Place),
                    "note" => Ok(ObjectType::Note),
                    "citation" => Ok(ObjectType::Citation),
                    "source" => Ok(ObjectType::Source),
                    "media" => Ok(ObjectType::Media),
                    "repository" => Ok(ObjectType::Repository),
                    "tag" => Ok(ObjectType::Tag),
                    _ => Err(E::custom(format!(
                        "unknown object_type \"{v}\", expected one of: {VALID_OBJECT_TYPES}"
                    ))),
                }
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<ObjectType, E> {
                Err(E::custom(format!(
                    "`object_type` is required, expected one of: {VALID_OBJECT_TYPES}"
                )))
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}

impl ObjectType {
    pub fn as_endpoint(self) -> &'static str {
        match self {
            Self::Person => "people",
            Self::Family => "families",
            Self::Event => "events",
            Self::Place => "places",
            Self::Note => "notes",
            Self::Citation => "citations",
            Self::Source => "sources",
            Self::Media => "media",
            Self::Repository => "repositories",
            Self::Tag => "tags",
        }
    }

    pub fn as_str(self) -> &'static str {
        self.into()
    }
}

#[derive(Deserialize, JsonSchema)]
pub struct SearchInput {
    pub query: String,
    /// Narrow search to a specific object type. Omit to search all types.
    pub object_type: Option<ObjectType>,
    /// Page number (1-based). Defaults to 1.
    pub page: Option<u32>,
    /// Results per page. Defaults to 20.
    pub pagesize: Option<u32>,
}

#[derive(Deserialize, JsonSchema, Debug)]
pub struct GetObjectInput {
    /// Required. One of: person, family, event, place, note, citation, source, media, repository, tag.
    pub object_type: ObjectType,
    /// Handle of a specific object — returns a single record.
    pub handle: Option<String>,
    /// Filter by Gramps ID (e.g. "I0001") — returns a collection.
    pub gramps_id: Option<String>,
    /// Page number (1-based) for paginated collection results.
    pub page: Option<u32>,
    /// Results per page for collection results (default 20).
    pub pagesize: Option<u32>,
}

#[derive(Deserialize, JsonSchema)]
pub struct HandleInput {
    pub handle: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct DeleteObjectInput {
    pub object_type: ObjectType,
    pub handle: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct CreatePersonInput {
    pub first_name: Option<String>,
    pub surname: Option<String>,
    /// Gender: 0=unknown, 1=male, 2=female
    pub gender: Option<i32>,
}

#[derive(Deserialize, JsonSchema)]
pub struct CreateFamilyInput {
    pub father_handle: Option<String>,
    pub mother_handle: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct CreateEventInput {
    pub event_type: String,
    pub description: Option<String>,
    pub date_text: Option<String>,
    pub place_handle: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct CreatePlaceInput {
    pub title: String,
    pub place_type: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct CreateSourceInput {
    pub title: String,
    pub author: Option<String>,
    pub pubinfo: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct CreateNoteInput {
    pub text: String,
    pub note_type: Option<String>,
}

/// survivor_handle (phoenix) survives; duplicate_handle (titanic) is deleted.
#[derive(Deserialize, JsonSchema)]
pub struct MergeInput {
    pub survivor_handle: String,
    pub duplicate_handle: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct MergePersonInput {
    pub survivor_handle: String,
    pub duplicate_handle: String,
    /// If true (default), also merge duplicate spouse/parent families.
    pub family_merger: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
pub struct MergeFamilyInput {
    pub survivor_handle: String,
    pub duplicate_handle: String,
    /// Handle of the person to keep as father. Defaults to survivor family's father.
    pub phoenix_father_handle: Option<String>,
    /// Handle of the person to keep as mother. Defaults to survivor family's mother.
    pub phoenix_mother_handle: Option<String>,
}

fn json_object_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({ "type": "object" })
}

#[derive(Deserialize, JsonSchema)]
pub struct UpdateInput {
    pub handle: String,
    /// The full object to replace the existing record with (must be a JSON object, not a string).
    #[schemars(schema_with = "json_object_schema")]
    pub data: serde_json::Value,
}

#[derive(Deserialize, JsonSchema)]
pub struct HandlePairInput {
    pub handle1: String,
    pub handle2: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct CreateTagInput {
    pub name: String,
    pub color: Option<String>,
    pub priority: Option<i32>,
}

#[derive(Deserialize, JsonSchema)]
pub struct CreateCitationInput {
    pub source_handle: String,
    pub page: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct CreateRepositoryInput {
    pub name: String,
    pub repo_type: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct CreateMediaInput {
    /// Path to an existing file on the Gramps server (relative to media directory).
    pub path: Option<String>,
    /// URL to download the file from and upload to Gramps.
    pub url: Option<String>,
    pub description: Option<String>,
    /// MIME type, e.g. "image/jpeg". Detected automatically for URL downloads.
    pub mime: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_object_null_type_returns_helpful_error() {
        let json = r#"{"object_type": null, "handle": "abc123"}"#;
        let msg = serde_json::from_str::<GetObjectInput>(json)
            .unwrap_err()
            .to_string();
        assert!(
            msg.contains("object_type"),
            "error should mention object_type: {msg}"
        );
        assert!(
            msg.contains("person"),
            "error should list valid values: {msg}"
        );
    }

    #[test]
    fn get_object_missing_type_returns_error() {
        let json = r#"{"handle": "abc123"}"#;
        assert!(serde_json::from_str::<GetObjectInput>(json).is_err());
    }

    #[test]
    fn get_object_valid_types_deserialize() {
        for (s, expected) in [
            ("person", ObjectType::Person),
            ("family", ObjectType::Family),
            ("event", ObjectType::Event),
            ("place", ObjectType::Place),
            ("note", ObjectType::Note),
            ("citation", ObjectType::Citation),
            ("source", ObjectType::Source),
            ("media", ObjectType::Media),
            ("repository", ObjectType::Repository),
            ("tag", ObjectType::Tag),
        ] {
            let json = format!(r#"{{"object_type": "{s}", "handle": "h"}}"#);
            let input: GetObjectInput = serde_json::from_str(&json).unwrap();
            assert_eq!(
                input.object_type.as_endpoint(),
                expected.as_endpoint(),
                "object_type \"{s}\" should deserialize correctly"
            );
        }
    }

    #[test]
    fn get_object_invalid_type_returns_helpful_error() {
        let json = r#"{"object_type": "dinosaur", "handle": "h"}"#;
        let msg = serde_json::from_str::<GetObjectInput>(json)
            .unwrap_err()
            .to_string();
        assert!(
            msg.contains("dinosaur"),
            "error should echo the bad value: {msg}"
        );
        assert!(
            msg.contains("person"),
            "error should list valid values: {msg}"
        );
    }

    #[test]
    fn update_input_accepts_json_object() {
        let json = r#"{"handle": "abc123", "data": {"gramps_id": "I0001"}}"#;
        let input: UpdateInput = serde_json::from_str(json).unwrap();
        assert!(input.data.is_object());
    }

    #[test]
    fn update_input_accepts_json_string_as_data() {
        // serde accepts strings (schema hint + runtime check in server.rs catches this)
        let json = r#"{"handle": "abc123", "data": "{\"gramps_id\":\"I0001\"}"}"#;
        let input: UpdateInput = serde_json::from_str(json).unwrap();
        assert!(input.data.is_string(), "deserialized as string, not object");
    }

    #[test]
    fn update_input_data_schema_is_object_type() {
        let schema = schemars::schema_for!(UpdateInput);
        let schema_json = serde_json::to_value(&schema).unwrap();
        let data_schema = &schema_json["properties"]["data"];
        assert_eq!(
            data_schema["type"].as_str(),
            Some("object"),
            "data field schema must constrain type to 'object', got: {data_schema}"
        );
    }
}
