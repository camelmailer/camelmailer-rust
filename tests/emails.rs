//! Wiremock tests for the emails service.

mod common;

use camelmailer_rs::{
    Attachment, Error, ListMessagesParams, SendEmailRequest, SendTemplateRequest,
};
use common::{client, error, success, API_KEY};
use serde_json::json;
use wiremock::matchers::{body_json, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn send_posts_message_and_parses_result() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/server/messages"))
        .and(header("X-Server-API-Key", API_KEY))
        .and(header("content-type", "application/json"))
        .and(body_json(json!({
            "from": "billing@acme.com",
            "to": ["ada@example.com"],
            "subject": "Your receipt",
            "text_body": "Thanks!",
            "tag": "receipt",
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(success(json!({
            "message_id": 1201,
            "recipients": [{
                "rcpt_to": "ada@example.com",
                "message_id": 1201,
                "token": "abcdef123456",
                "status": "queued",
            }],
        }))))
        .expect(1)
        .mount(&server)
        .await;

    let result = client(&server)
        .emails()
        .send(
            SendEmailRequest::builder()
                .from("billing@acme.com")
                .to("ada@example.com")
                .subject("Your receipt")
                .text_body("Thanks!")
                .tag("receipt")
                .build(),
        )
        .await
        .unwrap();

    assert_eq!(result.message_id, Some(1201));
    assert_eq!(result.recipients.len(), 1);
    assert_eq!(result.recipients[0].rcpt_to, "ada@example.com");
    assert_eq!(result.recipients[0].token, "abcdef123456");
    assert_eq!(result.recipients[0].status, "queued");
}

#[tokio::test]
async fn send_serializes_named_addresses_and_attachments() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/server/messages"))
        .and(body_json(json!({
            "from": { "email": "billing@acme.com", "name": "Acme Billing" },
            "to": ["ada@example.com"],
            "subject": "Invoice",
            "html_body": "<p>Hi</p>",
            "headers": { "X-Custom": "yes" },
            "attachments": [{
                "name": "hi.txt",
                "content_type": "text/plain",
                "data_base64": "aGVsbG8=",
            }],
            "metadata": { "order_id": 42 },
            "stream": "billing",
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(success(json!({
            "message_id": 7, "recipients": [],
        }))))
        .expect(1)
        .mount(&server)
        .await;

    client(&server)
        .emails()
        .send(
            SendEmailRequest::builder()
                .from(("billing@acme.com", "Acme Billing"))
                .to("ada@example.com")
                .subject("Invoice")
                .html_body("<p>Hi</p>")
                .header("X-Custom", "yes")
                .attachment(Attachment::from_bytes("hi.txt", "text/plain", b"hello"))
                .metadata(json!({ "order_id": 42 }))
                .stream("billing")
                .build(),
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn send_surfaces_validation_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/server/messages"))
        .respond_with(
            ResponseTemplate::new(422)
                .set_body_json(error("ValidationError", "From domain is not authorized")),
        )
        .mount(&server)
        .await;

    let err = client(&server)
        .emails()
        .send(
            SendEmailRequest::builder()
                .from("a@b.com")
                .to("c@d.com")
                .build(),
        )
        .await
        .unwrap_err();

    assert!(err.is_validation_error());
    assert_eq!(err.status(), Some(422));
    match err {
        Error::Api {
            code,
            message,
            status,
        } => {
            assert_eq!(code, "ValidationError");
            assert_eq!(message, "From domain is not authorized");
            assert_eq!(status, 422);
        }
        other => panic!("expected Error::Api, got {other:?}"),
    }
}

#[tokio::test]
async fn send_batch_posts_bare_array_and_splits_results() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/server/messages/batch"))
        .and(body_json(json!([
            { "from": "a@acme.com", "to": ["one@example.com"], "subject": "1" },
            { "from": "a@acme.com", "to": ["two@example.com"], "subject": "2" },
        ])))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
            "messages": [
                { "status": "success", "data": { "message_id": 1, "recipients": [] } },
                { "status": "error", "error": { "code": "ValidationError", "message": "no recipients" } },
            ],
        }))))
        .expect(1)
        .mount(&server)
        .await;

    let entries = client(&server)
        .emails()
        .send_batch(vec![
            SendEmailRequest::builder()
                .from("a@acme.com")
                .to("one@example.com")
                .subject("1")
                .build(),
            SendEmailRequest::builder()
                .from("a@acme.com")
                .to("two@example.com")
                .subject("2")
                .build(),
        ])
        .await
        .unwrap();

    assert_eq!(entries.len(), 2);
    assert!(entries[0].is_success());
    assert_eq!(entries[0].as_result().unwrap().message_id, Some(1));
    let entry_error = entries[1].as_result().unwrap_err();
    assert_eq!(entry_error.code(), Some("ValidationError"));
}

#[tokio::test]
async fn send_with_template_posts_template_fields() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/server/messages/with_template"))
        .and(body_json(json!({
            "template": "welcome",
            "template_model": { "name": "Ada" },
            "from": "hello@acme.com",
            "to": ["ada@example.com"],
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(success(json!({
            "message_id": 55,
            "recipients": [{
                "rcpt_to": "ada@example.com", "message_id": 55,
                "token": "tok", "status": "queued",
            }],
        }))))
        .expect(1)
        .mount(&server)
        .await;

    let result = client(&server)
        .emails()
        .send_with_template(
            SendTemplateRequest::builder("welcome")
                .from("hello@acme.com")
                .to("ada@example.com")
                .model(json!({ "name": "Ada" }))
                .build(),
        )
        .await
        .unwrap();

    assert_eq!(result.message_id, Some(55));
}

#[tokio::test]
async fn send_with_template_batch_posts_bare_array() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/server/messages/with_template/batch"))
        .and(body_json(json!([
            {
                "template": "welcome",
                "template_model": { "name": "Ada" },
                "to": ["ada@example.com"],
            },
        ])))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
            "messages": [
                { "status": "success", "data": { "message_id": 9, "recipients": [] } },
            ],
        }))))
        .expect(1)
        .mount(&server)
        .await;

    let entries = client(&server)
        .emails()
        .send_with_template_batch(vec![SendTemplateRequest::builder("welcome")
            .to("ada@example.com")
            .model(json!({ "name": "Ada" }))
            .build()])
        .await
        .unwrap();

    assert_eq!(entries.len(), 1);
    assert!(entries[0].is_success());
}

#[tokio::test]
async fn get_returns_message_with_deliveries() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/server/messages/1201"))
        .and(header("X-Server-API-Key", API_KEY))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
            "message": {
                "id": 1201,
                "token": "abcdef123456",
                "scope": "outgoing",
                "rcpt_to": "ada@example.com",
                "mail_from": "billing@acme.com",
                "subject": "Your receipt",
                "message_id": "<m1@acme.com>",
                "tag": "receipt",
                "status": "Sent",
                "bounce": false,
                "spam_status": null,
                "spam_score": 0.1,
                "held": false,
                "threat": false,
                "size": 2048,
                "metadata": { "order_id": 42 },
                "stream_id": 1,
                "bypassed": false,
                "created_at": "2026-07-11T09:00:00+00:00",
            },
            "deliveries": [{
                "id": 5,
                "status": "Sent",
                "details": "Accepted",
                "output": "250 OK",
                "sent_with_ssl": true,
                "created_at": "2026-07-11T09:00:05+00:00",
            }],
        }))))
        .mount(&server)
        .await;

    let detail = client(&server).emails().get(1201).await.unwrap();
    assert_eq!(detail.message.id, 1201);
    assert_eq!(detail.message.status.as_deref(), Some("Sent"));
    assert_eq!(detail.message.spam_score, Some(0.1));
    assert_eq!(detail.deliveries.len(), 1);
    assert!(detail.deliveries[0].sent_with_ssl);
}

#[tokio::test]
async fn get_missing_message_is_not_found() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/server/messages/999"))
        .respond_with(ResponseTemplate::new(404).set_body_json(error("NotFound", "not found")))
        .mount(&server)
        .await;

    let err = client(&server).emails().get(999).await.unwrap_err();
    assert!(err.is_not_found());
}

#[tokio::test]
async fn list_sends_all_filters_as_query_params() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/server/messages"))
        .and(query_param("page", "2"))
        .and(query_param("per_page", "50"))
        .and(query_param("scope", "outgoing"))
        .and(query_param("status", "Sent"))
        .and(query_param("tag", "receipt"))
        .and(query_param("query", "ada"))
        .and(query_param("stream", "billing"))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
            "messages": [{ "id": 1, "token": "t", "scope": "outgoing",
                            "rcpt_to": "ada@example.com", "bounce": false,
                            "held": false, "threat": false, "bypassed": false,
                            "created_at": "2026-07-11T09:00:00+00:00" }],
            "pagination": { "page": 2, "per_page": 50, "total": 51, "total_pages": 2 },
        }))))
        .expect(1)
        .mount(&server)
        .await;

    let list = client(&server)
        .emails()
        .list(
            ListMessagesParams::new()
                .page(2)
                .per_page(50)
                .scope("outgoing")
                .status("Sent")
                .tag("receipt")
                .query("ada")
                .stream("billing"),
        )
        .await
        .unwrap();

    assert_eq!(list.messages.len(), 1);
    assert_eq!(list.pagination.page, 2);
    assert_eq!(list.pagination.total, 51);
}

#[tokio::test]
async fn list_without_filters_sends_no_query() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/server/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
            "messages": [], "pagination": { "page": 1, "per_page": 30, "total": 0, "total_pages": 0 },
        }))))
        .mount(&server)
        .await;

    let list = client(&server)
        .emails()
        .list(ListMessagesParams::new())
        .await
        .unwrap();
    assert!(list.messages.is_empty());

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests[0].url.query(), None);
}

#[tokio::test]
async fn deliveries_opens_clicks_unwrap_their_lists() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/server/messages/7/deliveries"))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
            "deliveries": [{ "id": 1, "status": "SoftFail", "details": "greylisted",
                              "output": "451 try later", "sent_with_ssl": false,
                              "created_at": "2026-07-11T09:00:00+00:00" }],
        }))))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v2/server/messages/7/opens"))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
            "opens": [{ "ip_address": "203.0.113.9", "user_agent": "Mozilla/5.0",
                         "url": null, "created_at": "2026-07-11T10:00:00+00:00" }],
        }))))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v2/server/messages/7/clicks"))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
            "clicks": [{ "ip_address": "203.0.113.9", "user_agent": "Mozilla/5.0",
                          "url": "https://acme.com", "created_at": "2026-07-11T10:01:00+00:00" }],
        }))))
        .mount(&server)
        .await;

    let camelmailer = client(&server);
    let deliveries = camelmailer.emails().deliveries(7).await.unwrap();
    assert_eq!(deliveries[0].status, "SoftFail");
    let opens = camelmailer.emails().opens(7).await.unwrap();
    assert_eq!(opens[0].ip_address.as_deref(), Some("203.0.113.9"));
    let clicks = camelmailer.emails().clicks(7).await.unwrap();
    assert_eq!(clicks[0].url.as_deref(), Some("https://acme.com"));
}

#[tokio::test]
async fn raw_returns_decodable_source() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/server/messages/7/raw"))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
            "raw_message": "U3ViamVjdDogSGkNCg0KSGVsbG8h",
        }))))
        .mount(&server)
        .await;

    let raw = client(&server).emails().raw(7).await.unwrap();
    let bytes = raw.decode().unwrap();
    assert_eq!(bytes, b"Subject: Hi\r\n\r\nHello!");
}
