use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, JsonSchema)]
pub struct QueryInput {
    pub query: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct HandleInput {
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
