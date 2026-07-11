//! Wiremock tests for the stats and bounces services.

mod common;

use camelmailer_rs::{ListBouncesParams, StatsParams};
use common::{client, error, success};
use serde_json::json;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn stats_get_sends_window_and_parses_counters() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/server/stats"))
        .and(query_param("from", "2026-07-01T00:00:00Z"))
        .and(query_param("to", "2026-07-11T00:00:00Z"))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
            "stats": {
                "total": 100, "incoming": 10, "outgoing": 90,
                "sent": 85, "pending": 2, "held": 1,
                "bounced": 2, "soft_fail": 3, "hard_fail": 1,
                "opens": 40, "clicks": 12, "unique_opens": 30, "unique_clicks": 9,
            },
        }))))
        .expect(1)
        .mount(&server)
        .await;

    let stats = client(&server)
        .stats()
        .get(
            StatsParams::new()
                .from("2026-07-01T00:00:00Z")
                .to("2026-07-11T00:00:00Z"),
        )
        .await
        .unwrap();

    assert_eq!(stats.total, 100);
    assert_eq!(stats.sent, 85);
    assert_eq!(stats.unique_clicks, 9);
}

#[tokio::test]
async fn stats_deliveries_parses_queue_depth() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/server/stats/deliveries"))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
            "queued": 7,
            "domains": [
                { "domain": "example.com", "queued": 5 },
                { "domain": "example.org", "queued": 2 },
            ],
        }))))
        .mount(&server)
        .await;

    let stats = client(&server).stats().deliveries().await.unwrap();
    assert_eq!(stats.queued, 7);
    assert_eq!(stats.domains.len(), 2);
    assert_eq!(stats.domains[0].domain, "example.com");
    assert_eq!(stats.domains[0].queued, 5);
}

#[tokio::test]
async fn bounces_list_paginates() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/server/bounces"))
        .and(query_param("page", "1"))
        .and(query_param("per_page", "10"))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
            "bounces": [{
                "id": 77, "token": "tok", "scope": "incoming",
                "rcpt_to": "bounces@acme.com", "bounce": true,
                "held": false, "threat": false, "bypassed": false,
                "created_at": "2026-07-11T09:00:00+00:00",
            }],
            "pagination": { "page": 1, "per_page": 10, "total": 1, "total_pages": 1 },
        }))))
        .expect(1)
        .mount(&server)
        .await;

    let list = client(&server)
        .bounces()
        .list(ListBouncesParams::new().page(1).per_page(10))
        .await
        .unwrap();

    assert_eq!(list.bounces.len(), 1);
    assert!(list.bounces[0].bounce);
    assert_eq!(list.pagination.total, 1);
}

#[tokio::test]
async fn bounces_get_unwraps_bounce() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/server/bounces/77"))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
            "bounce": {
                "id": 77, "token": "tok", "scope": "incoming",
                "rcpt_to": "bounces@acme.com", "bounce": true,
                "held": false, "threat": false, "bypassed": false,
                "created_at": "2026-07-11T09:00:00+00:00",
            },
        }))))
        .mount(&server)
        .await;

    let bounce = client(&server).bounces().get(77).await.unwrap();
    assert_eq!(bounce.id, 77);
    assert!(bounce.bounce);
}

#[tokio::test]
async fn bounces_get_missing_is_not_found() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/server/bounces/1"))
        .respond_with(ResponseTemplate::new(404).set_body_json(error("NotFound", "not found")))
        .mount(&server)
        .await;

    let err = client(&server).bounces().get(1).await.unwrap_err();
    assert!(err.is_not_found());
}
