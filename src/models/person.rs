use super::{GrampsDate, Handle};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersonName {
    #[serde(rename = "type")]
    pub name_type: Option<String>,
    pub first_name: Option<String>,
    pub surname_list: Vec<Surname>,
    pub suffix: Option<String>,
    pub title: Option<String>,
    pub nick: Option<String>,
    pub call: Option<String>,
    pub date: Option<GrampsDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Surname {
    pub surname: Option<String>,
    pub prefix: Option<String>,
    pub primary: Option<bool>,
    pub origintype: Option<String>,
    pub connector: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Person {
    pub handle: Handle,
    pub gramps_id: Option<String>,
    pub gender: Option<i32>,
    pub primary_name: Option<PersonName>,
    pub alternate_names: Option<Vec<PersonName>>,
    pub event_ref_list: Option<Vec<serde_json::Value>>,
    pub family_list: Option<Vec<Handle>>,
    pub parent_family_list: Option<Vec<Handle>>,
    pub media_list: Option<Vec<serde_json::Value>>,
    pub address_list: Option<Vec<serde_json::Value>>,
    pub attribute_list: Option<Vec<serde_json::Value>>,
    pub urls: Option<Vec<serde_json::Value>>,
    pub lds_ord_list: Option<Vec<serde_json::Value>>,
    pub citation_list: Option<Vec<Handle>>,
    pub note_list: Option<Vec<Handle>>,
    pub change: Option<i64>,
    pub tag_list: Option<Vec<Handle>>,
    pub private: Option<bool>,
    pub person_ref_list: Option<Vec<serde_json::Value>>,
}

/// Request body for creating a new person.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreatePersonRequest {
    pub primary_name: Option<PersonName>,
    pub gender: Option<i32>,
    pub gramps_id: Option<String>,
    pub note_list: Option<Vec<Handle>>,
    pub citation_list: Option<Vec<Handle>>,
    pub event_ref_list: Option<Vec<serde_json::Value>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn person_deserializes_full_record() {
        let json = serde_json::json!({
            "handle": "ABCDEF123",
            "gramps_id": "I0001",
            "gender": 1,
            "primary_name": {
                "type": "Birth Name",
                "first_name": "John",
                "surname_list": [{"surname": "Smith", "primary": true}]
            }
        });
        let person: Person = serde_json::from_value(json).unwrap();
        assert_eq!(person.handle, "ABCDEF123");
        assert_eq!(person.gramps_id.as_deref(), Some("I0001"));
        assert_eq!(person.gender, Some(1));
        let name = person.primary_name.unwrap();
        assert_eq!(name.first_name.as_deref(), Some("John"));
        assert_eq!(name.name_type.as_deref(), Some("Birth Name"));
        let surname = &name.surname_list[0];
        assert_eq!(surname.surname.as_deref(), Some("Smith"));
        assert_eq!(surname.primary, Some(true));
    }

    #[test]
    fn person_deserializes_minimal_record() {
        let json = serde_json::json!({ "handle": "XYZ" });
        let person: Person = serde_json::from_value(json).unwrap();
        assert_eq!(person.handle, "XYZ");
        assert!(person.gender.is_none());
        assert!(person.primary_name.is_none());
        assert!(person.family_list.is_none());
    }

    #[test]
    fn create_person_request_serializes_name() {
        let req = CreatePersonRequest {
            primary_name: Some(PersonName {
                first_name: Some("Anna".into()),
                surname_list: vec![Surname {
                    surname: Some("Karenina".into()),
                    ..Default::default()
                }],
                name_type: Some("Birth Name".into()),
                ..Default::default()
            }),
            gender: Some(2),
            ..Default::default()
        };
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(v["gender"], 2);
        assert_eq!(v["primary_name"]["first_name"], "Anna");
        assert_eq!(v["primary_name"]["surname_list"][0]["surname"], "Karenina");
        assert_eq!(v["primary_name"]["type"], "Birth Name");
    }
}
