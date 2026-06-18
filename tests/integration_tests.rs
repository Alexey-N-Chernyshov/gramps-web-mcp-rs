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
    tools::{create, delete, get, update},
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

    delete::delete_media(client, &handle).await.unwrap();
    assert!(matches!(
        get::get_media(client, &handle).await,
        Err(Error::NotFound(_))
    ));
}
