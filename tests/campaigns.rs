//! Wiremock tests for the campaigns service.

mod common;

use camelmailer_rs::{CreateAndSendCampaign, CreateDraftCampaign, UpdateCampaign};
use common::{client, error, success};
use serde_json::json;
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn september() -> serde_json::Value {
    json!({
        "id": 7,
        "name": "September",
        "subject": "What shipped",
        "from": "news@acme.com",
        "status": "draft",
        "total": 120,
        "sent": 0,
        "stream_id": 3,
        "stream": { "permalink": "product-news", "name": "Product news" },
        "scheduled_at": null,
        "created_at": "2026-09-01T10:00:00Z",
        "completed_at": null,
    })
}

#[tokio::test]
async fn list_unwraps_campaigns() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/server/campaigns"))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
            "campaigns": [september()],
        }))))
        .mount(&server)
        .await;

    let campaigns = client(&server).campaigns().list().await.unwrap();
    assert_eq!(campaigns.len(), 1);
    assert_eq!(campaigns[0].stream.permalink, "product-news");
    assert!(campaigns[0].scheduled_at.is_none());
}

#[tokio::test]
async fn list_for_stream_uses_the_stream_path() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/server/streams/product-news/campaigns"))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({ "campaigns": [] }))))
        .mount(&server)
        .await;

    let campaigns = client(&server)
        .campaigns()
        .list_for_stream("product-news")
        .await
        .unwrap();
    assert!(campaigns.is_empty());
}

#[tokio::test]
async fn get_carries_the_stats() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/server/campaigns/7"))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
            "campaign": september(),
            "stats": {
                "total": 120, "sent": 118, "delivered": 110,
                "failed": 8, "opened": 40, "clicked": 9, "unsubscribed": 1,
            },
        }))))
        .mount(&server)
        .await;

    let detail = client(&server).campaigns().get(7).await.unwrap();
    assert_eq!(detail.stats.delivered, 110);
    assert_eq!(detail.campaign.id, 7);
}

#[tokio::test]
async fn get_for_stream_uses_the_stream_path() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/server/streams/product-news/campaigns/7"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(success(json!({ "campaign": september() }))),
        )
        .mount(&server)
        .await;

    let detail = client(&server)
        .campaigns()
        .get_for_stream("product-news", 7)
        .await
        .unwrap();
    assert_eq!(detail.campaign.id, 7);
}

#[tokio::test]
async fn create_draft_names_the_stream_in_the_body() {
    let server = MockServer::start().await;
    // The planning route: the stream travels in the body, not the path.
    Mock::given(method("POST"))
        .and(path("/api/v2/server/campaigns"))
        .and(body_json(json!({
            "stream": "product-news",
            "from": "news@acme.com",
            "name": "September",
        })))
        .respond_with(
            ResponseTemplate::new(201).set_body_json(success(json!({ "campaign": september() }))),
        )
        .mount(&server)
        .await;

    let campaign = client(&server)
        .campaigns()
        .create_draft(CreateDraftCampaign::new("product-news", "news@acme.com").name("September"))
        .await
        .unwrap();
    assert_eq!(campaign.status, "draft");
}

#[tokio::test]
async fn create_draft_arms_a_schedule() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/server/campaigns"))
        .and(body_json(json!({
            "stream": "product-news",
            "from": "news@acme.com",
            "scheduled_at": "2026-10-01T08:00:00Z",
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(success(json!({
            "campaign": { "id": 8, "status": "scheduled" },
        }))))
        .mount(&server)
        .await;

    let campaign = client(&server)
        .campaigns()
        .create_draft(
            CreateDraftCampaign::new("product-news", "news@acme.com")
                .scheduled_at("2026-10-01T08:00:00Z"),
        )
        .await
        .unwrap();
    assert_eq!(campaign.status, "scheduled");
}

#[tokio::test]
async fn create_and_send_uses_the_stream_route() {
    let server = MockServer::start().await;
    // The stream-scoped route expands to the subscribers before it
    // answers, so the campaign comes back already sending.
    Mock::given(method("POST"))
        .and(path("/api/v2/server/streams/product-news/campaigns"))
        .respond_with(ResponseTemplate::new(201).set_body_json(success(json!({
            "campaign": { "id": 9, "status": "sending" },
        }))))
        .mount(&server)
        .await;

    let campaign = client(&server)
        .campaigns()
        .create_and_send(
            "product-news",
            CreateAndSendCampaign::new("Status update").from("news@acme.com"),
        )
        .await
        .unwrap();
    assert_eq!(campaign.status, "sending");
}

#[tokio::test]
async fn update_schedules_with_a_time() {
    let server = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/api/v2/server/campaigns/7"))
        .and(body_json(json!({ "scheduled_at": "2026-10-01T08:00:00Z" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
            "campaign": { "id": 7, "status": "scheduled" },
        }))))
        .mount(&server)
        .await;

    let campaign = client(&server)
        .campaigns()
        .update(
            7,
            UpdateCampaign::new().scheduled_at("2026-10-01T08:00:00Z"),
        )
        .await
        .unwrap();
    assert_eq!(campaign.status, "scheduled");
}

#[tokio::test]
async fn clear_schedule_sends_an_explicit_null() {
    let server = MockServer::start().await;
    // An omitted field leaves the schedule standing; only an explicit
    // null drops the campaign back to a draft.
    Mock::given(method("PATCH"))
        .and(path("/api/v2/server/campaigns/7"))
        .and(body_json(json!({ "scheduled_at": null })))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
            "campaign": { "id": 7, "status": "draft" },
        }))))
        .mount(&server)
        .await;

    let campaign = client(&server)
        .campaigns()
        .update(7, UpdateCampaign::new().clear_schedule())
        .await
        .unwrap();
    assert_eq!(campaign.status, "draft");
}

#[tokio::test]
async fn an_untouched_schedule_is_omitted() {
    let server = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/api/v2/server/campaigns/7"))
        .and(body_json(json!({ "subject": "Corrected" })))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(success(json!({ "campaign": september() }))),
        )
        .mount(&server)
        .await;

    client(&server)
        .campaigns()
        .update(7, UpdateCampaign::new().subject("Corrected"))
        .await
        .unwrap();
}

#[tokio::test]
async fn send_and_cancel() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/server/campaigns/7/send"))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
            "campaign": { "id": 7, "status": "sending" },
        }))))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v2/server/campaigns/7/cancel"))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
            "campaign": { "id": 7, "status": "canceled" },
        }))))
        .mount(&server)
        .await;

    let client = client(&server);
    assert_eq!(client.campaigns().send(7).await.unwrap().status, "sending");
    assert_eq!(
        client.campaigns().cancel(7).await.unwrap().status,
        "canceled"
    );
}

#[tokio::test]
async fn editing_a_sending_campaign_is_refused() {
    let server = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/api/v2/server/campaigns/7"))
        .respond_with(ResponseTemplate::new(422).set_body_json(error(
            "ValidationError",
            "a sent campaign can no longer be edited",
        )))
        .mount(&server)
        .await;

    let err = client(&server)
        .campaigns()
        .update(7, UpdateCampaign::new().subject("Too late"))
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        camelmailer_rs::Error::Api { ref code, .. } if code == "ValidationError"
    ));
}
