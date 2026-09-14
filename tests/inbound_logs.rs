//! Wiremock tests for the inbound and logs services.

mod common;

use camelmailer_rs::{ListInboundParams, ListLogsParams};
use common::{client, success};
use serde_json::json;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn inbound_list_reads_the_inbound_key() {
    let server = MockServer::start().await;
    // The page comes back under "inbound", not "messages".
    Mock::given(method("GET"))
        .and(path("/api/v2/server/inbound"))
        .and(query_param("status", "held"))
        .and(query_param("per_page", "50"))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
            "inbound": [{ "id": 55, "status": "Held", "held": true }],
            "pagination": { "page": 1, "per_page": 50, "total": 1, "total_pages": 1 },
        }))))
        .mount(&server)
        .await;

    let page = client(&server)
        .inbound()
        .list(ListInboundParams::new().status("held").per_page(50))
        .await
        .unwrap();
    assert_eq!(page.inbound.len(), 1);
    assert!(page.inbound[0].held);
    assert_eq!(page.pagination.total, 1);
}

#[tokio::test]
async fn inbound_get_unwraps_the_message() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/server/inbound/55"))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
            "message": { "id": 55, "status": "Held", "held": true },
        }))))
        .mount(&server)
        .await;

    let message = client(&server).inbound().get(55).await.unwrap();
    assert_eq!(message.id, 55);
}

#[tokio::test]
async fn inbound_retry_and_bypass() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/server/inbound/55/retry"))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({ "queued": true }))))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v2/server/inbound/55/bypass"))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({ "queued": true }))))
        .mount(&server)
        .await;

    let client = client(&server);
    assert!(client.inbound().retry(55).await.unwrap().queued);
    assert!(client.inbound().bypass(55).await.unwrap().queued);
}

#[tokio::test]
async fn logs_list_and_tags() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/server/logs"))
        .and(query_param("per_page", "25"))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
            "requests": [{
                "id": 1, "method": "POST", "path": "/api/v2/server/messages",
                "status_code": 201, "duration_ms": 12, "user_agent": "camelmailer-rs",
                "created_at": "2026-09-14T08:00:00Z",
            }],
            "pagination": { "page": 1, "per_page": 25, "total": 1, "total_pages": 1 },
        }))))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v2/server/tags"))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
            "tags": [{ "tag": "receipt", "count": 12 }],
        }))))
        .mount(&server)
        .await;

    let client = client(&server);
    let page = client
        .logs()
        .list(ListLogsParams::new().per_page(25))
        .await
        .unwrap();
    assert_eq!(page.requests[0].status_code, 201);
    assert_eq!(page.requests[0].duration_ms, 12);

    let tags = client.logs().tags().await.unwrap();
    assert_eq!(tags[0].tag, "receipt");
    assert_eq!(tags[0].count, 12);
}
