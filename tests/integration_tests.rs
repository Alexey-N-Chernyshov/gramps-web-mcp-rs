mod common;

use gramps_mcp_rs::{
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
    let result = get::get_person(&fixture.client, "NONEXISTENT_HANDLE").await;
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

    let person = get::get_person(&fixture.client, &handle).await.unwrap();
    assert_eq!(person.handle, handle);
    assert_eq!(
        person.primary_name.unwrap().first_name.as_deref(),
        Some("Ivan")
    );
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

    let person = get::get_person(client, &handle).await.unwrap();
    assert_eq!(
        person.primary_name.as_ref().unwrap().first_name.as_deref(),
        Some("Ivan")
    );

    let mut body = serde_json::to_value(&person).unwrap();
    body["primary_name"]["first_name"] = serde_json::json!("Petr");
    update::update_person(client, &handle, &body).await.unwrap();

    let updated = get::get_person(client, &handle).await.unwrap();
    assert_eq!(
        updated.primary_name.unwrap().first_name.as_deref(),
        Some("Petr")
    );

    delete::delete_person(client, &handle).await.unwrap();
    assert!(matches!(
        get::get_person(client, &handle).await,
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

    let family = get::get_family(client, &family_handle).await.unwrap();
    assert_eq!(family.father_handle.as_deref(), Some(father.as_str()));
    assert_eq!(family.mother_handle.as_deref(), Some(mother.as_str()));
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

    let family = get::get_family(client, &family_handle).await.unwrap();
    let children = family.child_ref_list.unwrap();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].ref_handle.as_deref(), Some(child.as_str()));
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

    let event = get::get_event(client, &handle).await.unwrap();
    assert_eq!(event.description.as_deref(), Some("Test birth event"));
    assert!(event.event_type.is_some());

    let mut body = serde_json::to_value(&event).unwrap();
    body["description"] = serde_json::json!("Updated description");
    update::update_event(client, &handle, &body).await.unwrap();
    let updated = get::get_event(client, &handle).await.unwrap();
    assert_eq!(updated.description.as_deref(), Some("Updated description"));

    delete::delete_event(client, &handle).await.unwrap();
    assert!(matches!(
        get::get_event(client, &handle).await,
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

    let source = get::get_source(client, &handle).await.unwrap();
    assert_eq!(source.title.as_deref(), Some("Vital Records 1850"));
    assert_eq!(source.author.as_deref(), Some("County Office"));

    let mut body = serde_json::to_value(&source).unwrap();
    body["title"] = serde_json::json!("Vital Records 1900");
    update::update_source(client, &handle, &body).await.unwrap();
    let updated = get::get_source(client, &handle).await.unwrap();
    assert_eq!(updated.title.as_deref(), Some("Vital Records 1900"));

    delete::delete_source(client, &handle).await.unwrap();
    assert!(matches!(
        get::get_source(client, &handle).await,
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

    let citation = get::get_citation(client, &citation_handle).await.unwrap();
    assert_eq!(
        citation.source_handle.as_deref(),
        Some(source_handle.as_str())
    );
    assert_eq!(citation.page.as_deref(), Some("p. 42"));

    let mut body = serde_json::to_value(&citation).unwrap();
    body["page"] = serde_json::json!("p. 99");
    update::update_citation(client, &citation_handle, &body)
        .await
        .unwrap();
    let updated = get::get_citation(client, &citation_handle).await.unwrap();
    assert_eq!(updated.page.as_deref(), Some("p. 99"));

    delete::delete_citation(client, &citation_handle)
        .await
        .unwrap();
    delete::delete_source(client, &source_handle).await.unwrap();
}

#[tokio::test]
async fn note_round_trip() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let handle = create::create_note(client, "Hello from test", Some("General"))
        .await
        .unwrap();

    let note = get::get_note(client, &handle).await.unwrap();
    let text_str = note.text.as_ref().and_then(|t| t["string"].as_str());
    assert_eq!(text_str, Some("Hello from test"));

    let mut body = serde_json::to_value(&note).unwrap();
    body["text"]["string"] = serde_json::json!("Updated note text");
    update::update_note(client, &handle, &body).await.unwrap();
    let updated = get::get_note(client, &handle).await.unwrap();
    assert_eq!(
        updated.text.as_ref().and_then(|t| t["string"].as_str()),
        Some("Updated note text")
    );

    delete::delete_note(client, &handle).await.unwrap();
    assert!(matches!(
        get::get_note(client, &handle).await,
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

    let place = get::get_place(client, &handle).await.unwrap();
    assert_eq!(place.title.as_deref(), Some("Moscow"));

    let mut body = serde_json::to_value(&place).unwrap();
    body["title"] = serde_json::json!("Saint Petersburg");
    update::update_place(client, &handle, &body).await.unwrap();
    let updated = get::get_place(client, &handle).await.unwrap();
    assert_eq!(updated.title.as_deref(), Some("Saint Petersburg"));

    delete::delete_place(client, &handle).await.unwrap();
    assert!(matches!(
        get::get_place(client, &handle).await,
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

    let tag = get::get_tag(client, &handle).await.unwrap();
    assert_eq!(tag.name.as_deref(), Some("Important"));
    assert_eq!(tag.color.as_deref(), Some("#FF0000"));

    let mut body = serde_json::to_value(&tag).unwrap();
    body["color"] = serde_json::json!("#00FF00");
    update::update_tag(client, &handle, &body).await.unwrap();
    let updated = get::get_tag(client, &handle).await.unwrap();
    assert_eq!(updated.color.as_deref(), Some("#00FF00"));

    delete::delete_tag(client, &handle).await.unwrap();
    assert!(matches!(
        get::get_tag(client, &handle).await,
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

    let repo = get::get_repository(client, &handle).await.unwrap();
    assert_eq!(repo.name.as_deref(), Some("National Archives"));
    assert_eq!(repo.repo_type.as_deref(), Some("Archive"));

    let mut body = serde_json::to_value(&repo).unwrap();
    body["name"] = serde_json::json!("State Archives");
    update::update_repository(client, &handle, &body)
        .await
        .unwrap();
    let updated = get::get_repository(client, &handle).await.unwrap();
    assert_eq!(updated.name.as_deref(), Some("State Archives"));

    delete::delete_repository(client, &handle).await.unwrap();
    assert!(matches!(
        get::get_repository(client, &handle).await,
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

    let media = get::get_media(client, &handle).await.unwrap();
    assert_eq!(media["handle"].as_str(), Some(handle.as_str()));
    assert_eq!(media["path"].as_str(), Some("/photos/test.jpg"));
    assert_eq!(media["desc"].as_str(), Some("Test photo"));

    let mut body = media.clone();
    body["desc"] = serde_json::json!("Updated photo");
    update::update_media(client, &handle, &body).await.unwrap();
    let updated = get::get_media(client, &handle).await.unwrap();
    assert_eq!(updated["desc"].as_str(), Some("Updated photo"));

    delete::delete_media(client, &handle).await.unwrap();
    assert!(matches!(
        get::get_media(client, &handle).await,
        Err(Error::NotFound(_))
    ));
}

#[tokio::test]
async fn media_from_url_round_trip() {
    use tokio::io::AsyncWriteExt as _;

    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    // Serve a tiny JPEG locally — no external deps, guaranteed non-empty response
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

    let media = get::get_media(client, &handle).await.unwrap();
    assert_eq!(media["handle"].as_str(), Some(handle.as_str()));
    assert_eq!(media["desc"].as_str(), Some("Downloaded photo"));
    assert_eq!(media["mime"].as_str(), Some("image/jpeg"));

    let mut body = media.clone();
    body["desc"] = serde_json::json!("Updated downloaded photo");
    update::update_media(client, &handle, &body).await.unwrap();
    let updated = get::get_media(client, &handle).await.unwrap();
    assert_eq!(updated["desc"].as_str(), Some("Updated downloaded photo"));

    delete::delete_media(client, &handle).await.unwrap();
    assert!(matches!(
        get::get_media(client, &handle).await,
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

    delete::delete_person(client, &handle).await.unwrap();
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

    delete::delete_family(client, &family_handle).await.unwrap();
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

    // The endpoint returns an object (or null) when no path is found, not necessarily an array.
    get::get_relations(client, &father, &child).await.unwrap();

    delete::delete_family(client, &family_handle).await.unwrap();
    delete::delete_person(client, &father).await.unwrap();
    delete::delete_person(client, &child).await.unwrap();
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

    delete::delete_event(client, &handle1).await.unwrap();
    delete::delete_event(client, &handle2).await.unwrap();
}

#[tokio::test]
async fn search_endpoints_return_ok() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    // Create one object of each searchable type so the index is non-empty
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

    // Each find_* endpoint must be reachable (Ok) and return a JSON array.
    // If a type is not yet indexed, the server returns Ok([]) — still valid.
    macro_rules! assert_search {
        ($call:expr, $name:literal) => {
            let r = $call.await.expect(concat!($name, " failed"));
            assert!(r.is_array(), concat!($name, " should return an array"));
        };
    }

    assert_search!(search::find_person(client, "Searchable"), "find_person");
    assert_search!(search::find_source(client, "Search Source"), "find_source");
    assert_search!(search::find_citation(client, "citation"), "find_citation");
    assert_search!(search::find_event(client, "Birth"), "find_event");
    assert_search!(search::find_place(client, "Search Place"), "find_place");
    assert_search!(search::find_family(client, "family"), "find_family");
    assert_search!(search::find_note(client, "note"), "find_note");
    assert_search!(search::find_tag(client, "SearchTag"), "find_tag");
    assert_search!(
        search::find_repository(client, "Search Repo"),
        "find_repository"
    );
    assert_search!(search::find_media(client, "search"), "find_media");
    assert_search!(search::find_anything(client, "Search"), "find_anything");

    // Cleanup
    delete::delete_person(client, &person).await.unwrap();
    delete::delete_citation(client, &citation).await.unwrap();
    delete::delete_source(client, &source).await.unwrap();
    delete::delete_event(client, &event).await.unwrap();
    delete::delete_place(client, &place).await.unwrap();
    delete::delete_family(client, &family).await.unwrap();
    delete::delete_note(client, &note).await.unwrap();
    delete::delete_tag(client, &tag).await.unwrap();
    delete::delete_repository(client, &repo).await.unwrap();
    delete::delete_media(client, &media).await.unwrap();
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
        get::get_person(client, &p2).await,
        Err(Error::NotFound(_))
    ));
    delete::delete_person(client, &p1).await.unwrap();

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
        get::get_family(client, &f2).await,
        Err(Error::NotFound(_))
    ));
    delete::delete_family(client, &f1).await.unwrap();

    // merge_event
    let e1 = create::create_event(client, CreateEventRequest::default())
        .await
        .unwrap();
    let e2 = create::create_event(client, CreateEventRequest::default())
        .await
        .unwrap();
    merge::merge_event(client, &e1, &e2).await.unwrap();
    assert!(matches!(
        get::get_event(client, &e2).await,
        Err(Error::NotFound(_))
    ));
    delete::delete_event(client, &e1).await.unwrap();

    // merge_place
    let pl1 = create::create_place(client, CreatePlaceRequest::default())
        .await
        .unwrap();
    let pl2 = create::create_place(client, CreatePlaceRequest::default())
        .await
        .unwrap();
    merge::merge_place(client, &pl1, &pl2).await.unwrap();
    assert!(matches!(
        get::get_place(client, &pl2).await,
        Err(Error::NotFound(_))
    ));
    delete::delete_place(client, &pl1).await.unwrap();

    // merge_note
    let n1 = create::create_note(client, "note one", None).await.unwrap();
    let n2 = create::create_note(client, "note two", None).await.unwrap();
    merge::merge_note(client, &n1, &n2).await.unwrap();
    assert!(matches!(
        get::get_note(client, &n2).await,
        Err(Error::NotFound(_))
    ));
    delete::delete_note(client, &n1).await.unwrap();

    // merge_source + merge_citation + merge_repository (need source first)
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
        get::get_citation(client, &c2).await,
        Err(Error::NotFound(_))
    ));
    delete::delete_citation(client, &c1).await.unwrap();

    merge::merge_source(client, &src1, &src2).await.unwrap();
    assert!(matches!(
        get::get_source(client, &src2).await,
        Err(Error::NotFound(_))
    ));
    delete::delete_source(client, &src1).await.unwrap();

    let r1 = create::create_repository(client, "Repo One", None)
        .await
        .unwrap();
    let r2 = create::create_repository(client, "Repo Two", None)
        .await
        .unwrap();
    merge::merge_repository(client, &r1, &r2).await.unwrap();
    assert!(matches!(
        get::get_repository(client, &r2).await,
        Err(Error::NotFound(_))
    ));
    delete::delete_repository(client, &r1).await.unwrap();

    // merge_media
    let m1 = create::create_media_from_path(client, "/tmp/a.jpg", None, None)
        .await
        .unwrap();
    let m2 = create::create_media_from_path(client, "/tmp/b.jpg", None, None)
        .await
        .unwrap();
    merge::merge_media(client, &m1, &m2).await.unwrap();
    assert!(matches!(
        get::get_media(client, &m2).await,
        Err(Error::NotFound(_))
    ));
    delete::delete_media(client, &m1).await.unwrap();
}
