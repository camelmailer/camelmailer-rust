//! Wiremock tests for the blocking client (`blocking` feature).
#![cfg(feature = "blocking")]

mod common;

use camelmailer_rs::blocking::CamelMailer;
use camelmailer_rs::{ListMessagesParams, SendEmailRequest};
use common::{error, success, API_KEY};
use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// The blocking client owns its own runtime, so the mock server must
/// live on a separate one.
fn mock_server() -> (tokio::runtime::Runtime, MockServer) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();
    let server = runtime.block_on(MockServer::start());
    (runtime, server)
}

fn blocking_client(server: &MockServer) -> CamelMailer {
    CamelMailer::builder(API_KEY)
        .base_url(server.uri())
        .build()
        .unwrap()
}

#[test]
fn blocking_send_works() {
    let (_runtime, server) = mock_server();
    _runtime.block_on(
        Mock::given(method("POST"))
            .and(path("/api/v2/server/messages"))
            .and(header("X-Server-API-Key", API_KEY))
            .respond_with(ResponseTemplate::new(201).set_body_json(success(json!({
                "message_id": 3, "recipients": [],
            }))))
            .mount(&server),
    );

    let result = blocking_client(&server)
        .emails()
        .send(
            SendEmailRequest::builder()
                .from("billing@acme.com")
                .to("ada@example.com")
                .subject("Hi")
                .text_body("Hello")
                .build(),
        )
        .unwrap();
    assert_eq!(result.message_id, Some(3));
}

#[test]
fn blocking_list_and_ping_work() {
    let (_runtime, server) = mock_server();
    _runtime.block_on(async {
        Mock::given(method("GET"))
            .and(path("/api/v2/server/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
                "messages": [],
                "pagination": { "page": 1, "per_page": 30, "total": 0, "total_pages": 0 },
            }))))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/v2/server/ping"))
            .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
                "pong": true, "server_id": 4, "server": "acme-prod",
            }))))
            .mount(&server)
            .await;
    });

    let camelmailer = blocking_client(&server);
    let list = camelmailer
        .emails()
        .list(ListMessagesParams::new())
        .unwrap();
    assert!(list.messages.is_empty());
    let ping = camelmailer.server().ping().unwrap();
    assert!(ping.pong);
}

#[test]
fn blocking_errors_are_typed() {
    let (_runtime, server) = mock_server();
    _runtime.block_on(
        Mock::given(method("GET"))
            .and(path("/api/v2/server/ping"))
            .respond_with(
                ResponseTemplate::new(401).set_body_json(error("Unauthorized", "bad key")),
            )
            .mount(&server),
    );

    let err = blocking_client(&server).server().ping().unwrap_err();
    assert!(err.is_unauthorized());
}
