use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, JsonSchema, Clone, Copy, strum::IntoStaticStr)]
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

#[derive(Deserialize, JsonSchema)]
pub struct GetObjectInput {
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

#[derive(Deserialize, JsonSchema)]
pub struct UpdateInput {
    pub handle: String,
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
