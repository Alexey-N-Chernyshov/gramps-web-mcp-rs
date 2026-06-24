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

mod common;

use gramps_web_mcp_rs::{
    client::{Error, GrampsClient},
    config::Config,
    models::{
        event::CreateEventRequest,
        family::CreateFamilyRequest,
        person::{CreatePersonRequest, PersonName, Surname},
        place::{CreatePlaceRequest, PlaceName},
        source::CreateSourceRequest,
    },
    tools::{create, delete, get, merge, search, update},
};

#[tokio::test]
async fn auth_fails_with_wrong_password() {
    let fixture = common::TestFixture::new().await;
    let bad_client = GrampsClient::new(
        Config {
            gramps_api_url: fixture.base_url.clone(),
            gramps_username: common::TEST_USER.to_string(),
            gramps_password: "wrongpassword".to_string(),
            gramps_readonly: false,
            mcp_transport: Default::default(),
            mcp_http_host: Default::default(),
            mcp_http_port: Default::default(),
            mcp_auth_token: None,
            mcp_allowed_hosts: None,
        },
        reqwest::Client::new(),
    );
    let result = get::get_tree_info(&bad_client).await;
    assert!(
        matches!(result, Err(Error::Auth(_))),
        "expected Auth error, got: {result:?}"
    );
}

#[tokio::test]
async fn get_nonexistent_returns_not_found() {
    let fixture = common::TestFixture::new().await;
    let result = get::get_object_by_handle(&fixture.client, "people", "NONEXISTENT_HANDLE").await;
    assert!(
        matches!(result, Err(Error::NotFound(_))),
        "expected NotFound, got: {result:?}"
    );
}

#[tokio::test]
async fn delete_nonexistent_returns_not_found() {
    let fixture = common::TestFixture::new().await;
    let result = delete::delete_object(&fixture.client, "people", "NONEXISTENT_HANDLE").await;
    assert!(
        matches!(result, Err(Error::NotFound(_))),
        "expected NotFound, got: {result:?}"
    );
}

#[tokio::test]
async fn create_and_get_person_round_trip() {
    let fixture = common::TestFixture::new().await;

    let handle = create::create_person(
        &fixture.client,
        CreatePersonRequest {
            primary_name: Some(PersonName {
                first_name: Some("Ivan".to_string()),
                surname_list: vec![Surname {
                    surname: Some("Petrov".to_string()),
                    ..Default::default()
                }],
                name_type: Some("Birth Name".to_string()),
                ..Default::default()
            }),
            gender: Some(1),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let person = get::get_object_by_handle(&fixture.client, "people", &handle)
        .await
        .unwrap();
    assert_eq!(person["handle"].as_str(), Some(handle.as_str()));
    assert_eq!(person["primary_name"]["first_name"].as_str(), Some("Ivan"));
}

#[tokio::test]
async fn person_lifecycle() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let handle = create::create_person(
        client,
        CreatePersonRequest {
            primary_name: Some(PersonName {
                first_name: Some("Ivan".to_string()),
                surname_list: vec![Surname {
                    surname: Some("Sidorov".to_string()),
                    ..Default::default()
                }],
                name_type: Some("Birth Name".to_string()),
                ..Default::default()
            }),
            gender: Some(1),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let person = get::get_object_by_handle(client, "people", &handle)
        .await
        .unwrap();
    assert_eq!(person["primary_name"]["first_name"].as_str(), Some("Ivan"));

    let mut body = person.clone();
    body["primary_name"]["first_name"] = serde_json::json!("Petr");
    update::update_person(client, &handle, &body).await.unwrap();

    let updated = get::get_object_by_handle(client, "people", &handle)
        .await
        .unwrap();
    assert_eq!(updated["primary_name"]["first_name"].as_str(), Some("Petr"));

    delete::delete_object(client, "people", &handle)
        .await
        .unwrap();
    assert!(matches!(
        get::get_object_by_handle(client, "people", &handle).await,
        Err(Error::NotFound(_))
    ));
}

#[tokio::test]
async fn family_with_parents() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let father = create::create_person(
        client,
        CreatePersonRequest {
            gender: Some(1),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let mother = create::create_person(
        client,
        CreatePersonRequest {
            gender: Some(2),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let family_handle = create::create_family(
        client,
        CreateFamilyRequest {
            father_handle: Some(father.clone()),
            mother_handle: Some(mother.clone()),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let family = get::get_object_by_handle(client, "families", &family_handle)
        .await
        .unwrap();
    assert_eq!(family["father_handle"].as_str(), Some(father.as_str()));
    assert_eq!(family["mother_handle"].as_str(), Some(mother.as_str()));

    let new_mother = create::create_person(
        client,
        CreatePersonRequest {
            gender: Some(2),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let mut body = family.clone();
    body["mother_handle"] = serde_json::json!(new_mother);
    update::update_family(client, &family_handle, &body)
        .await
        .unwrap();
    let updated = get::get_object_by_handle(client, "families", &family_handle)
        .await
        .unwrap();
    assert_eq!(
        updated["mother_handle"].as_str(),
        Some(new_mother.as_str()),
        "mother_handle should be updated"
    );
    delete::delete_object(client, "people", &new_mother)
        .await
        .unwrap();
}

#[tokio::test]
async fn family_with_child() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let child = create::create_person(client, CreatePersonRequest::default())
        .await
        .unwrap();

    let family_handle = create::create_family(
        client,
        CreateFamilyRequest {
            child_ref_list: Some(vec![serde_json::json!({"ref": child})]),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let family = get::get_object_by_handle(client, "families", &family_handle)
        .await
        .unwrap();
    let children = family["child_ref_list"].as_array().unwrap();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0]["ref"].as_str(), Some(child.as_str()));
}

#[tokio::test]
async fn event_round_trip() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let handle = create::create_event(
        client,
        CreateEventRequest {
            event_type: Some(serde_json::json!("Birth")),
            description: Some("Test birth event".to_string()),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let event = get::get_object_by_handle(client, "events", &handle)
        .await
        .unwrap();
    assert_eq!(event["description"].as_str(), Some("Test birth event"));
    assert!(!event["type"].is_null());

    let mut body = event.clone();
    body["description"] = serde_json::json!("Updated description");
    update::update_event(client, &handle, &body).await.unwrap();
    let updated = get::get_object_by_handle(client, "events", &handle)
        .await
        .unwrap();
    assert_eq!(updated["description"].as_str(), Some("Updated description"));

    delete::delete_object(client, "events", &handle)
        .await
        .unwrap();
    assert!(matches!(
        get::get_object_by_handle(client, "events", &handle).await,
        Err(Error::NotFound(_))
    ));
}

#[tokio::test]
async fn source_round_trip() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let handle = create::create_source(
        client,
        CreateSourceRequest {
            title: Some("Vital Records 1850".to_string()),
            author: Some("County Office".to_string()),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let source = get::get_object_by_handle(client, "sources", &handle)
        .await
        .unwrap();
    assert_eq!(source["title"].as_str(), Some("Vital Records 1850"));
    assert_eq!(source["author"].as_str(), Some("County Office"));

    let mut body = source.clone();
    body["title"] = serde_json::json!("Vital Records 1900");
    update::update_source(client, &handle, &body).await.unwrap();
    let updated = get::get_object_by_handle(client, "sources", &handle)
        .await
        .unwrap();
    assert_eq!(updated["title"].as_str(), Some("Vital Records 1900"));

    delete::delete_object(client, "sources", &handle)
        .await
        .unwrap();
    assert!(matches!(
        get::get_object_by_handle(client, "sources", &handle).await,
        Err(Error::NotFound(_))
    ));
}

#[tokio::test]
async fn citation_links_source() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let source_handle = create::create_source(
        client,
        CreateSourceRequest {
            title: Some("Parish Records".to_string()),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let citation_handle = create::create_citation(client, &source_handle, Some("p. 42"))
        .await
        .unwrap();

    let citation = get::get_object_by_handle(client, "citations", &citation_handle)
        .await
        .unwrap();
    assert_eq!(
        citation["source_handle"].as_str(),
        Some(source_handle.as_str())
    );
    assert_eq!(citation["page"].as_str(), Some("p. 42"));

    let mut body = citation.clone();
    body["page"] = serde_json::json!("p. 99");
    update::update_citation(client, &citation_handle, &body)
        .await
        .unwrap();
    let updated = get::get_object_by_handle(client, "citations", &citation_handle)
        .await
        .unwrap();
    assert_eq!(updated["page"].as_str(), Some("p. 99"));

    delete::delete_object(client, "citations", &citation_handle)
        .await
        .unwrap();
    delete::delete_object(client, "sources", &source_handle)
        .await
        .unwrap();
}

#[tokio::test]
async fn note_round_trip() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let handle = create::create_note(client, "Hello from test", Some("General"))
        .await
        .unwrap();

    let note = get::get_object_by_handle(client, "notes", &handle)
        .await
        .unwrap();
    assert_eq!(note["text"]["string"].as_str(), Some("Hello from test"));

    let mut body = note.clone();
    body["text"]["string"] = serde_json::json!("Updated note text");
    update::update_note(client, &handle, &body).await.unwrap();
    let updated = get::get_object_by_handle(client, "notes", &handle)
        .await
        .unwrap();
    assert_eq!(
        updated["text"]["string"].as_str(),
        Some("Updated note text")
    );

    delete::delete_object(client, "notes", &handle)
        .await
        .unwrap();
    assert!(matches!(
        get::get_object_by_handle(client, "notes", &handle).await,
        Err(Error::NotFound(_))
    ));
}

#[tokio::test]
async fn place_round_trip() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let handle = create::create_place(
        client,
        CreatePlaceRequest {
            title: Some("Moscow".to_string()),
            name: Some(PlaceName {
                value: Some("Moscow".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let place = get::get_object_by_handle(client, "places", &handle)
        .await
        .unwrap();
    assert_eq!(place["title"].as_str(), Some("Moscow"));

    let mut body = place.clone();
    body["title"] = serde_json::json!("Saint Petersburg");
    update::update_place(client, &handle, &body).await.unwrap();
    let updated = get::get_object_by_handle(client, "places", &handle)
        .await
        .unwrap();
    assert_eq!(updated["title"].as_str(), Some("Saint Petersburg"));

    delete::delete_object(client, "places", &handle)
        .await
        .unwrap();
    assert!(matches!(
        get::get_object_by_handle(client, "places", &handle).await,
        Err(Error::NotFound(_))
    ));
}

#[tokio::test]
async fn tag_round_trip() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let handle = create::create_tag(client, "Important", Some("#FF0000"), Some(1))
        .await
        .unwrap();

    let tag = get::get_object_by_handle(client, "tags", &handle)
        .await
        .unwrap();
    assert_eq!(tag["name"].as_str(), Some("Important"));
    assert_eq!(tag["color"].as_str(), Some("#FF0000"));

    let mut body = tag.clone();
    body["color"] = serde_json::json!("#00FF00");
    update::update_tag(client, &handle, &body).await.unwrap();
    let updated = get::get_object_by_handle(client, "tags", &handle)
        .await
        .unwrap();
    assert_eq!(updated["color"].as_str(), Some("#00FF00"));

    delete::delete_object(client, "tags", &handle)
        .await
        .unwrap();
    assert!(matches!(
        get::get_object_by_handle(client, "tags", &handle).await,
        Err(Error::NotFound(_))
    ));
}

#[tokio::test]
async fn repository_round_trip() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let handle = create::create_repository(client, "National Archives", Some("Archive"))
        .await
        .unwrap();

    let repo = get::get_object_by_handle(client, "repositories", &handle)
        .await
        .unwrap();
    assert_eq!(repo["name"].as_str(), Some("National Archives"));
    assert_eq!(repo["type"].as_str(), Some("Archive"));

    let mut body = repo.clone();
    body["name"] = serde_json::json!("State Archives");
    update::update_repository(client, &handle, &body)
        .await
        .unwrap();
    let updated = get::get_object_by_handle(client, "repositories", &handle)
        .await
        .unwrap();
    assert_eq!(updated["name"].as_str(), Some("State Archives"));

    delete::delete_object(client, "repositories", &handle)
        .await
        .unwrap();
    assert!(matches!(
        get::get_object_by_handle(client, "repositories", &handle).await,
        Err(Error::NotFound(_))
    ));
}

#[tokio::test]
async fn media_from_path_round_trip() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let handle = create::create_media_from_path(
        client,
        "/photos/test.jpg",
        Some("Test photo"),
        Some("image/jpeg"),
    )
    .await
    .unwrap();

    let media = get::get_object_by_handle(client, "media", &handle)
        .await
        .unwrap();
    assert_eq!(media["handle"].as_str(), Some(handle.as_str()));
    assert_eq!(media["path"].as_str(), Some("/photos/test.jpg"));
    assert_eq!(media["desc"].as_str(), Some("Test photo"));

    let mut body = media.clone();
    body["desc"] = serde_json::json!("Updated photo");
    update::update_media(client, &handle, &body).await.unwrap();
    let updated = get::get_object_by_handle(client, "media", &handle)
        .await
        .unwrap();
    assert_eq!(updated["desc"].as_str(), Some("Updated photo"));

    delete::delete_object(client, "media", &handle)
        .await
        .unwrap();
    assert!(matches!(
        get::get_object_by_handle(client, "media", &handle).await,
        Err(Error::NotFound(_))
    ));
}

#[tokio::test]
async fn media_from_url_round_trip() {
    use tokio::io::AsyncWriteExt as _;

    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let file_url = format!("http://127.0.0.1:{port}/photo.jpg");

    tokio::spawn(async move {
        if let Ok((mut sock, _)) = listener.accept().await {
            let _ = sock
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type: image/jpeg\r\nContent-Length: 3\r\n\r\nABC",
                )
                .await;
        }
    });

    let handle = create::create_media_from_url(client, &file_url, Some("Downloaded photo"), None)
        .await
        .unwrap();

    let media = get::get_object_by_handle(client, "media", &handle)
        .await
        .unwrap();
    assert_eq!(media["handle"].as_str(), Some(handle.as_str()));
    assert_eq!(media["desc"].as_str(), Some("Downloaded photo"));
    assert_eq!(media["mime"].as_str(), Some("image/jpeg"));

    // FAILED: path/checksum empty after POST /api/media/ with raw bytes — confirms
    // the create-then-PUT-file two-step pattern is required, see gramps-web-api #189/#273.
    let path = media["path"].as_str().unwrap_or("");
    assert!(
        !path.is_empty(),
        "media[\"path\"] is empty after create_media_from_url (got {:?}) — \
         file was not actually uploaded, only the metadata record was created",
        media["path"]
    );

    let checksum = media["checksum"].as_str().unwrap_or("");
    assert!(
        !checksum.is_empty(),
        "media[\"checksum\"] is empty after create_media_from_url (got {:?}) — \
         file was not actually uploaded, only the metadata record was created",
        media["checksum"]
    );

    let mut body = media.clone();
    body["desc"] = serde_json::json!("Updated downloaded photo");
    update::update_media(client, &handle, &body).await.unwrap();
    let updated = get::get_object_by_handle(client, "media", &handle)
        .await
        .unwrap();
    assert_eq!(updated["desc"].as_str(), Some("Updated downloaded photo"));

    delete::delete_object(client, "media", &handle)
        .await
        .unwrap();
    assert!(matches!(
        get::get_object_by_handle(client, "media", &handle).await,
        Err(Error::NotFound(_))
    ));
}

#[tokio::test]
async fn person_timeline() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let handle = create::create_person(client, CreatePersonRequest::default())
        .await
        .unwrap();

    let timeline = get::get_person_timeline(client, &handle).await.unwrap();
    assert!(timeline.is_array(), "timeline should be an array");

    delete::delete_object(client, "people", &handle)
        .await
        .unwrap();
}

#[tokio::test]
async fn family_timeline() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let family_handle = create::create_family(client, CreateFamilyRequest::default())
        .await
        .unwrap();

    let timeline = get::get_family_timeline(client, &family_handle)
        .await
        .unwrap();
    assert!(timeline.is_array(), "timeline should be an array");

    delete::delete_object(client, "families", &family_handle)
        .await
        .unwrap();
}

#[tokio::test]
async fn relations_between_parent_and_child() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let father = create::create_person(
        client,
        CreatePersonRequest {
            gender: Some(1),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let child = create::create_person(client, CreatePersonRequest::default())
        .await
        .unwrap();

    let family_handle = create::create_family(
        client,
        CreateFamilyRequest {
            father_handle: Some(father.clone()),
            child_ref_list: Some(vec![serde_json::json!({"ref": child})]),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    get::get_relations(client, &father, &child).await.unwrap();

    delete::delete_object(client, "families", &family_handle)
        .await
        .unwrap();
    delete::delete_object(client, "people", &father)
        .await
        .unwrap();
    delete::delete_object(client, "people", &child)
        .await
        .unwrap();
}

#[tokio::test]
async fn event_span_between_two_events() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let handle1 = create::create_event(
        client,
        CreateEventRequest {
            event_type: Some(serde_json::json!("Birth")),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let handle2 = create::create_event(
        client,
        CreateEventRequest {
            event_type: Some(serde_json::json!("Death")),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    get::get_event_span(client, &handle1, &handle2)
        .await
        .unwrap();

    delete::delete_object(client, "events", &handle1)
        .await
        .unwrap();
    delete::delete_object(client, "events", &handle2)
        .await
        .unwrap();
}

#[tokio::test]
async fn search_endpoints_return_ok() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let person = create::create_person(
        client,
        CreatePersonRequest {
            primary_name: Some(PersonName {
                first_name: Some("Searchable".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let source = create::create_source(
        client,
        CreateSourceRequest {
            title: Some("Search Source".to_string()),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let citation = create::create_citation(client, &source, Some("p. 1"))
        .await
        .unwrap();

    let event = create::create_event(
        client,
        CreateEventRequest {
            event_type: Some(serde_json::json!("Birth")),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let place = create::create_place(
        client,
        CreatePlaceRequest {
            title: Some("Search Place".to_string()),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let family = create::create_family(client, CreateFamilyRequest::default())
        .await
        .unwrap();

    let note = create::create_note(client, "search note text", None)
        .await
        .unwrap();

    let tag = create::create_tag(client, "SearchTag", None, None)
        .await
        .unwrap();

    let repo = create::create_repository(client, "Search Repo", None)
        .await
        .unwrap();

    let media = create::create_media_from_path(client, "/tmp/search.jpg", None, None)
        .await
        .unwrap();

    // Each type must be reachable and return a JSON array.
    // If a type is not yet indexed, the server returns Ok([]) — still valid.
    macro_rules! assert_search {
        ($query:expr, $type:expr) => {
            let r = search::search(client, $query, $type, None, None)
                .await
                .unwrap_or_else(|e| panic!("search({:?}, {:?}) failed: {e}", $query, $type));
            assert!(
                r.is_array(),
                "search({:?}, {:?}) should return an array",
                $query,
                $type
            );
        };
    }

    assert_search!("Searchable", Some("person"));
    assert_search!("Search Source", Some("source"));
    assert_search!("citation", Some("citation"));
    assert_search!("Birth", Some("event"));
    assert_search!("Search Place", Some("place"));
    assert_search!("family", Some("family"));
    assert_search!("note", Some("note"));
    assert_search!("SearchTag", Some("tag"));
    assert_search!("Search Repo", Some("repository"));
    assert_search!("search", Some("media"));
    assert_search!("Search", None::<&str>);

    // Cleanup
    delete::delete_object(client, "people", &person)
        .await
        .unwrap();
    delete::delete_object(client, "citations", &citation)
        .await
        .unwrap();
    delete::delete_object(client, "sources", &source)
        .await
        .unwrap();
    delete::delete_object(client, "events", &event)
        .await
        .unwrap();
    delete::delete_object(client, "places", &place)
        .await
        .unwrap();
    delete::delete_object(client, "families", &family)
        .await
        .unwrap();
    delete::delete_object(client, "notes", &note).await.unwrap();
    delete::delete_object(client, "tags", &tag).await.unwrap();
    delete::delete_object(client, "repositories", &repo)
        .await
        .unwrap();
    delete::delete_object(client, "media", &media)
        .await
        .unwrap();
}

#[tokio::test]
async fn get_object_collection_pagination_and_gramps_id() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let h1 = create::create_person(client, CreatePersonRequest::default())
        .await
        .unwrap();
    let h2 = create::create_person(client, CreatePersonRequest::default())
        .await
        .unwrap();

    // plain collection browse returns an array
    let all = get::get_object_collection(client, "people", None, None, None, None)
        .await
        .unwrap();
    assert!(
        all.is_array(),
        "collection without params should be an array"
    );

    // pagination: page=1 pagesize=1 must return at most 1 item
    let page1 = get::get_object_collection(client, "people", None, None, Some(1), Some(1))
        .await
        .unwrap();
    assert!(page1.is_array());
    assert!(
        page1.as_array().unwrap().len() <= 1,
        "pagesize=1 should return at most 1 item"
    );

    // gramps_id lookup: fetch the gramps_id of h1 then query by it
    let obj = get::get_object_by_handle(client, "people", &h1)
        .await
        .unwrap();
    let gramps_id = obj["gramps_id"].as_str().expect("gramps_id missing");
    let by_id = get::get_object_collection(client, "people", Some(gramps_id), None, None, None)
        .await
        .unwrap();
    assert!(by_id.is_array());
    let items = by_id.as_array().unwrap();
    assert_eq!(
        items.len(),
        1,
        "gramps_id lookup should return exactly 1 item"
    );
    assert_eq!(items[0]["handle"].as_str(), Some(h1.as_str()));

    delete::delete_object(client, "people", &h1).await.unwrap();
    delete::delete_object(client, "people", &h2).await.unwrap();
}

#[tokio::test]
async fn merge_operations() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    // merge_person
    let p1 = create::create_person(client, CreatePersonRequest::default())
        .await
        .unwrap();
    let p2 = create::create_person(client, CreatePersonRequest::default())
        .await
        .unwrap();
    merge::merge_person(client, &p1, &p2, false).await.unwrap();
    assert!(matches!(
        get::get_object_by_handle(client, "people", &p2).await,
        Err(Error::NotFound(_))
    ));
    delete::delete_object(client, "people", &p1).await.unwrap();

    // merge_family
    let f1 = create::create_family(client, CreateFamilyRequest::default())
        .await
        .unwrap();
    let f2 = create::create_family(client, CreateFamilyRequest::default())
        .await
        .unwrap();
    merge::merge_family(client, &f1, &f2, None, None)
        .await
        .unwrap();
    assert!(matches!(
        get::get_object_by_handle(client, "families", &f2).await,
        Err(Error::NotFound(_))
    ));
    delete::delete_object(client, "families", &f1)
        .await
        .unwrap();

    // merge_event
    let e1 = create::create_event(client, CreateEventRequest::default())
        .await
        .unwrap();
    let e2 = create::create_event(client, CreateEventRequest::default())
        .await
        .unwrap();
    merge::merge_event(client, &e1, &e2).await.unwrap();
    assert!(matches!(
        get::get_object_by_handle(client, "events", &e2).await,
        Err(Error::NotFound(_))
    ));
    delete::delete_object(client, "events", &e1).await.unwrap();

    // merge_place
    let pl1 = create::create_place(client, CreatePlaceRequest::default())
        .await
        .unwrap();
    let pl2 = create::create_place(client, CreatePlaceRequest::default())
        .await
        .unwrap();
    merge::merge_place(client, &pl1, &pl2).await.unwrap();
    assert!(matches!(
        get::get_object_by_handle(client, "places", &pl2).await,
        Err(Error::NotFound(_))
    ));
    delete::delete_object(client, "places", &pl1).await.unwrap();

    // merge_note
    let n1 = create::create_note(client, "note one", None).await.unwrap();
    let n2 = create::create_note(client, "note two", None).await.unwrap();
    merge::merge_note(client, &n1, &n2).await.unwrap();
    assert!(matches!(
        get::get_object_by_handle(client, "notes", &n2).await,
        Err(Error::NotFound(_))
    ));
    delete::delete_object(client, "notes", &n1).await.unwrap();

    // merge_source + merge_citation + merge_repository
    let src1 = create::create_source(
        client,
        CreateSourceRequest {
            title: Some("Source One".to_string()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let src2 = create::create_source(
        client,
        CreateSourceRequest {
            title: Some("Source Two".to_string()),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let c1 = create::create_citation(client, &src1, None).await.unwrap();
    let c2 = create::create_citation(client, &src2, None).await.unwrap();
    merge::merge_citation(client, &c1, &c2).await.unwrap();
    assert!(matches!(
        get::get_object_by_handle(client, "citations", &c2).await,
        Err(Error::NotFound(_))
    ));
    delete::delete_object(client, "citations", &c1)
        .await
        .unwrap();

    merge::merge_source(client, &src1, &src2).await.unwrap();
    assert!(matches!(
        get::get_object_by_handle(client, "sources", &src2).await,
        Err(Error::NotFound(_))
    ));
    delete::delete_object(client, "sources", &src1)
        .await
        .unwrap();

    let r1 = create::create_repository(client, "Repo One", None)
        .await
        .unwrap();
    let r2 = create::create_repository(client, "Repo Two", None)
        .await
        .unwrap();
    merge::merge_repository(client, &r1, &r2).await.unwrap();
    assert!(matches!(
        get::get_object_by_handle(client, "repositories", &r2).await,
        Err(Error::NotFound(_))
    ));
    delete::delete_object(client, "repositories", &r1)
        .await
        .unwrap();

    // merge_media
    let m1 = create::create_media_from_path(client, "/tmp/a.jpg", None, None)
        .await
        .unwrap();
    let m2 = create::create_media_from_path(client, "/tmp/b.jpg", None, None)
        .await
        .unwrap();
    merge::merge_media(client, &m1, &m2).await.unwrap();
    assert!(matches!(
        get::get_object_by_handle(client, "media", &m2).await,
        Err(Error::NotFound(_))
    ));
    delete::delete_object(client, "media", &m1).await.unwrap();
}

#[tokio::test]
async fn search_pagination() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    // Create two people with the same distinctive name so search finds at least 2 results.
    let h1 = create::create_person(
        client,
        CreatePersonRequest {
            primary_name: Some(PersonName {
                first_name: Some("Pagination".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let h2 = create::create_person(
        client,
        CreatePersonRequest {
            primary_name: Some(PersonName {
                first_name: Some("Pagination".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    // pagesize=1 must return at most 1 result
    let page1 = search::search(client, "Pagination", Some("person"), Some(1), Some(1))
        .await
        .unwrap();
    assert!(page1.is_array(), "paginated search should return an array");
    assert!(
        page1.as_array().unwrap().len() <= 1,
        "pagesize=1 should return at most 1 result, got {}",
        page1.as_array().unwrap().len()
    );

    // page=2 pagesize=1 — second page should also be an array (may be empty if not yet indexed)
    let page2 = search::search(client, "Pagination", Some("person"), Some(2), Some(1))
        .await
        .unwrap();
    assert!(page2.is_array(), "page 2 should return an array");

    // No pagination params — should also be fine (backward compat)
    let all = search::search(client, "Pagination", Some("person"), None, None)
        .await
        .unwrap();
    assert!(
        all.is_array(),
        "search without pagination should return an array"
    );

    delete::delete_object(client, "people", &h1).await.unwrap();
    delete::delete_object(client, "people", &h2).await.unwrap();
}

#[tokio::test]
async fn gql_filter_people_by_surname() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let handle = create::create_person(
        client,
        CreatePersonRequest {
            primary_name: Some(PersonName {
                surname_list: vec![Surname {
                    surname: Some("GqlTestSurname".to_string()),
                    ..Default::default()
                }],
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let result = get::get_object_collection(
        client,
        "people",
        None,
        Some(r#"primary_name.surname_list[0].surname ~ "GqlTestSurname""#),
        None,
        None,
    )
    .await
    .unwrap();

    assert!(result.is_array(), "gql filter should return an array");
    let items = result.as_array().unwrap();
    assert!(
        items
            .iter()
            .any(|p| p["handle"].as_str() == Some(handle.as_str())),
        "gql filter should find the created person"
    );

    // gql and pagesize work together
    let paged = get::get_object_collection(
        client,
        "people",
        None,
        Some(r#"primary_name.surname_list[0].surname ~ "GqlTestSurname""#),
        Some(1),
        Some(1),
    )
    .await
    .unwrap();
    assert!(paged.is_array(), "gql + pagesize=1 should return an array");
    assert!(
        paged.as_array().unwrap().len() <= 1,
        "pagesize=1 should cap results"
    );

    delete::delete_object(client, "people", &handle)
        .await
        .unwrap();
}

#[tokio::test]
async fn gql_filter_families_by_child_count() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let child = create::create_person(client, CreatePersonRequest::default())
        .await
        .unwrap();

    let empty_family = create::create_family(client, CreateFamilyRequest::default())
        .await
        .unwrap();

    let family_with_child = create::create_family(
        client,
        CreateFamilyRequest {
            child_ref_list: Some(vec![serde_json::json!({"ref": child})]),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let result = get::get_object_collection(
        client,
        "families",
        None,
        Some("child_ref_list.length > 0"),
        None,
        None,
    )
    .await
    .unwrap();

    assert!(result.is_array(), "gql filter should return an array");
    let items = result.as_array().unwrap();
    assert!(
        items
            .iter()
            .any(|f| f["handle"].as_str() == Some(family_with_child.as_str())),
        "family with child should be in results"
    );
    assert!(
        !items
            .iter()
            .any(|f| f["handle"].as_str() == Some(empty_family.as_str())),
        "family without children should not be in results"
    );

    delete::delete_object(client, "families", &empty_family)
        .await
        .unwrap();
    delete::delete_object(client, "families", &family_with_child)
        .await
        .unwrap();
    delete::delete_object(client, "people", &child)
        .await
        .unwrap();
}
