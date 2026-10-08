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

#[path = "support/helpers.rs"]
mod common;

use gramps_web_mcp_rs::{
    models::person::{CreatePersonRequest, PersonName},
    tools::{create, delete, get, query},
};

#[tokio::test]
async fn query_filter_people_by_gramps_id() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let handle = create::create_person(client, CreatePersonRequest::default())
        .await
        .unwrap();

    let obj = get::get_object_by_handle(client, "people", &handle)
        .await
        .unwrap();
    let gramps_id = obj["gramps_id"].as_str().expect("gramps_id missing");

    let where_expr = format!(r#"gramps_id == "{gramps_id}""#);
    let result = query::query_object(
        client,
        "people",
        None,
        Some(where_expr),
        None,
        None,
        None,
        None,
    )
    .await
    .unwrap();

    let items = result["items"]
        .as_array()
        .expect("query_object response should have an `items` array");
    assert!(
        items
            .iter()
            .any(|p| p["handle"].as_str() == Some(handle.as_str())),
        "where_expr filter should find the created person"
    );

    delete::delete_object(client, "people", &handle)
        .await
        .unwrap();
}

#[tokio::test]
async fn query_limit_caps_results() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let h1 = create::create_person(
        client,
        CreatePersonRequest {
            primary_name: Some(PersonName {
                first_name: Some("QueryLimitOne".to_string()),
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
                first_name: Some("QueryLimitTwo".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let result = query::query_object(client, "people", None, None, None, Some(1), None, None)
        .await
        .unwrap();

    let items = result["items"]
        .as_array()
        .expect("query_object response should have an `items` array");
    assert!(items.len() <= 1, "limit=1 should cap results to at most 1");

    delete::delete_object(client, "people", &h1).await.unwrap();
    delete::delete_object(client, "people", &h2).await.unwrap();
}

#[tokio::test]
async fn query_select_narrows_columns() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let handle = create::create_person(client, CreatePersonRequest::default())
        .await
        .unwrap();

    let where_expr = format!(r#"handle == "{handle}""#);
    let result = query::query_object(
        client,
        "people",
        Some(vec!["handle".to_string(), "gramps_id".to_string()]),
        Some(where_expr),
        None,
        None,
        None,
        None,
    )
    .await
    .unwrap();

    let items = result["items"]
        .as_array()
        .expect("query_object response should have an `items` array");
    assert_eq!(
        items.len(),
        1,
        "select+where_expr should find exactly one person"
    );
    assert_eq!(items[0]["handle"].as_str(), Some(handle.as_str()));

    delete::delete_object(client, "people", &handle)
        .await
        .unwrap();
}

#[tokio::test]
async fn query_count_returns_total_count() {
    let fixture = common::TestFixture::new().await;
    let client = &fixture.client;

    let handle = create::create_person(client, CreatePersonRequest::default())
        .await
        .unwrap();

    let where_expr = format!(r#"handle == "{handle}""#);
    let result = query::query_object(
        client,
        "people",
        None,
        Some(where_expr),
        None,
        None,
        None,
        Some(true),
    )
    .await
    .unwrap();

    assert_eq!(
        result["total_count"].as_u64(),
        Some(1),
        "count=true should report exactly one matching row"
    );

    delete::delete_object(client, "people", &handle)
        .await
        .unwrap();
}
