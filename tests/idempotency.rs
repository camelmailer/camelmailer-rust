//! Wiremock tests for idempotent sending and the broadcast send.

mod common;

use camelmailer_rs::{SendEmailRequest, SendTemplateRequest, SendToStreamRequest};
use common::{client, error, success};
use serde_json::json;
use wiremock::matchers::{body_json, header, header_exists, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn receipt() -> SendEmailRequest {
    SendEmailRequest::builder()
        .from("billing@acme.com")
        .to("ada@example.com")
        .subject("Your receipt")
        .text_body("Thanks.")
        .build()
}

#[tokio::test]
async fn the_key_travels_as_a_header_not_in_the_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/server/messages"))
        .and(header("Idempotency-Key", "order-4711"))
        // The body is what the server hashes for the claim, so the key
        // must not end up inside it.
        .and(body_json(json!({
            "from": "billing@acme.com",
            "to": ["ada@example.com"],
            "subject": "Your receipt",
            "text_body": "Thanks.",
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({ "message_id": 1 }))))
        .mount(&server)
        .await;

    client(&server)
        .emails()
        .idempotent("order-4711")
        .send(receipt())
        .await
        .unwrap();
}

#[tokio::test]
async fn no_header_without_a_key() {
    let server = MockServer::start().await;
    // A mock that only matches when the header is absent: the
    // header_exists matcher is negated by mounting it as the 404 path.
    Mock::given(method("POST"))
        .and(path("/api/v2/server/messages"))
        .and(header_exists("Idempotency-Key"))
        .respond_with(ResponseTemplate::new(500).set_body_json(error(
            "Unexpected",
            "an Idempotency-Key was sent without one being asked for",
        )))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v2/server/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({ "message_id": 1 }))))
        .mount(&server)
        .await;

    let result = client(&server).emails().send(receipt()).await.unwrap();
    assert_eq!(result.message_id, Some(1));
}

#[tokio::test]
async fn every_send_endpoint_carries_the_key() {
    // The API claims all four, so all four have to send it.
    for endpoint in [
        "/api/v2/server/messages",
        "/api/v2/server/messages/batch",
        "/api/v2/server/messages/with_template",
        "/api/v2/server/messages/with_template/batch",
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(endpoint))
            .and(header("Idempotency-Key", "k"))
            .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
                "message_id": 1,
                "messages": [],
            }))))
            .mount(&server)
            .await;

        let client = client(&server);
        let emails = client.emails().idempotent("k");
        let template = SendTemplateRequest::builder("welcome")
            .from("billing@acme.com")
            .to("ada@example.com")
            .build();
        match endpoint {
            "/api/v2/server/messages" => {
                emails.send(receipt()).await.unwrap();
            }
            "/api/v2/server/messages/batch" => {
                emails.send_batch([receipt()]).await.unwrap();
            }
            "/api/v2/server/messages/with_template" => {
                emails.send_with_template(template).await.unwrap();
            }
            _ => {
                emails.send_with_template_batch([template]).await.unwrap();
            }
        }
    }
}

#[tokio::test]
async fn a_reused_key_for_another_body_is_refused() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/server/messages"))
        .respond_with(ResponseTemplate::new(409).set_body_json(error(
            "InvalidIdempotentRequest",
            "The same idempotency key was used with a different request",
        )))
        .mount(&server)
        .await;

    let err = client(&server)
        .emails()
        .idempotent("reused")
        .send(receipt())
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        camelmailer_rs::Error::Api { ref code, status, .. }
            if code == "InvalidIdempotentRequest" && status == 409
    ));
}

#[tokio::test]
async fn the_send_allowance_surfaces_as_send_limit_exceeded() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/server/messages"))
        .respond_with(
            ResponseTemplate::new(429)
                .set_body_json(error("SendLimitExceeded", "the send allowance is used up")),
        )
        .mount(&server)
        .await;

    let err = client(&server).emails().send(receipt()).await.unwrap_err();
    assert!(matches!(
        err,
        camelmailer_rs::Error::Api { ref code, status, .. }
            if code == "SendLimitExceeded" && status == 429
    ));
}

#[tokio::test]
async fn send_to_stream_counts_queued_against_skipped() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/server/streams/newsletter/send"))
        .and(body_json(json!({
            "from": "news@acme.com",
            "subject": "September",
            "text_body": "Hello.",
        })))
        .respond_with(ResponseTemplate::new(202).set_body_json(success(json!({
            "queued": 42, "skipped": 3,
        }))))
        .mount(&server)
        .await;

    let result = client(&server)
        .emails()
        .send_to_stream(
            "newsletter",
            SendToStreamRequest::new("news@acme.com")
                .subject("September")
                .text_body("Hello."),
        )
        .await
        .unwrap();
    assert_eq!(result.queued, 42);
    assert_eq!(result.skipped, 3);
}
