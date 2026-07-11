//! Wiremock tests for the streams service.

mod common;

use camelmailer_rs::{CreateStreamRequest, UpdateStreamRequest};
use common::{client, error, success};
use serde_json::json;
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn billing_stream() -> serde_json::Value {
    json!({
        "id": 2,
        "uuid": "11111111-0000-0000-0000-000000000000",
        "name": "Billing",
        "permalink": "billing",
        "stream_type": "transactional",
        "archived": false,
    })
}

#[tokio::test]
async fn list_unwraps_streams() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/server/streams"))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
            "streams": [billing_stream()],
        }))))
        .mount(&server)
        .await;

    let streams = client(&server).streams().list().await.unwrap();
    assert_eq!(streams.len(), 1);
    assert_eq!(streams[0].stream_type, "transactional");
}

#[tokio::test]
async fn create_posts_name_and_type() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/server/streams"))
        .and(body_json(
            json!({ "name": "Billing", "stream_type": "transactional" }),
        ))
        .respond_with(ResponseTemplate::new(201).set_body_json(success(json!({
            "stream": billing_stream(),
        }))))
        .expect(1)
        .mount(&server)
        .await;

    let stream = client(&server)
        .streams()
        .create(CreateStreamRequest::new("Billing").stream_type("transactional"))
        .await
        .unwrap();
    assert_eq!(stream.permalink, "billing");
}

#[tokio::test]
async fn create_with_invalid_type_is_validation_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/server/streams"))
        .respond_with(ResponseTemplate::new(422).set_body_json(error(
            "ValidationError",
            "Stream type \"nope\" is not valid",
        )))
        .mount(&server)
        .await;

    let err = client(&server)
        .streams()
        .create(CreateStreamRequest::new("Billing").stream_type("nope"))
        .await
        .unwrap_err();
    assert!(err.is_validation_error());
}

#[tokio::test]
async fn get_fetches_by_permalink() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/server/streams/billing"))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
            "stream": billing_stream(),
        }))))
        .mount(&server)
        .await;

    let stream = client(&server).streams().get("billing").await.unwrap();
    assert_eq!(stream.name, "Billing");
}

#[tokio::test]
async fn update_patches_fields() {
    let server = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/api/v2/server/streams/billing"))
        .and(body_json(
            json!({ "name": "Billing v2", "archived": false }),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
            "stream": billing_stream(),
        }))))
        .expect(1)
        .mount(&server)
        .await;

    client(&server)
        .streams()
        .update(
            "billing",
            UpdateStreamRequest::new()
                .name("Billing v2")
                .archived(false),
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn archive_posts_to_archive_path() {
    let server = MockServer::start().await;
    let mut archived = billing_stream();
    archived["archived"] = json!(true);
    Mock::given(method("POST"))
        .and(path("/api/v2/server/streams/billing/archive"))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
            "stream": archived,
        }))))
        .mount(&server)
        .await;

    let stream = client(&server).streams().archive("billing").await.unwrap();
    assert!(stream.archived);
}
