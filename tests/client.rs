//! Wiremock tests for the client itself: auth, server service, error paths.

mod common;

use camelmailer_rs::Error;
use common::{client, error, success, API_KEY};
use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn ping_sends_api_key_header() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/server/ping"))
        .and(header("X-Server-API-Key", API_KEY))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
            "pong": true, "server_id": 4, "server": "acme-prod",
        }))))
        .expect(1)
        .mount(&server)
        .await;

    let ping = client(&server).server().ping().await.unwrap();
    assert!(ping.pong);
    assert_eq!(ping.server_id, 4);
    assert_eq!(ping.server, "acme-prod");
}

#[tokio::test]
async fn server_get_parses_settings() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/server/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
            "server": {
                "id": 4,
                "uuid": "22222222-0000-0000-0000-000000000000",
                "name": "Acme Production",
                "permalink": "acme-prod",
                "mode": "Live",
                "suspended": false,
                "suspension_reason": null,
                "privacy_mode": false,
                "track_opens": true,
                "track_clicks": true,
                "spam_threshold": 5.0,
                "outbound_spam_threshold": 4.0,
                "bounce_hook_url": null,
                "delivery_hook_url": null,
                "inbound_domain": "in.acme.com",
                "color": "#336699",
                "ip_pool_id": null,
                "default_stream_id": 1,
            },
        }))))
        .mount(&server)
        .await;

    let details = client(&server).server().get().await.unwrap();
    assert_eq!(details.name, "Acme Production");
    assert_eq!(details.mode, "Live");
    assert!(details.track_opens);
    assert_eq!(details.default_stream_id, Some(1));
}

#[tokio::test]
async fn invalid_key_is_unauthorized() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/server/ping"))
        .respond_with(
            ResponseTemplate::new(401)
                .set_body_json(error("Unauthorized", "invalid or missing API key")),
        )
        .mount(&server)
        .await;

    let err = client(&server).server().ping().await.unwrap_err();
    assert!(err.is_unauthorized());
    assert_eq!(err.code(), Some("Unauthorized"));
    assert_eq!(err.status(), Some(401));
}

#[tokio::test]
async fn non_envelope_body_is_unexpected_response() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/server/ping"))
        .respond_with(ResponseTemplate::new(502).set_body_string("<html>Bad gateway</html>"))
        .mount(&server)
        .await;

    let err = client(&server).server().ping().await.unwrap_err();
    match err {
        Error::UnexpectedResponse { status, body } => {
            assert_eq!(status, 502);
            assert!(body.contains("Bad gateway"));
        }
        other => panic!("expected UnexpectedResponse, got {other:?}"),
    }
}

#[tokio::test]
async fn unreachable_host_is_network_error() {
    // Nothing listens on this port.
    let camelmailer = camelmailer_rs::CamelMailer::builder("cm_test")
        .base_url("http://127.0.0.1:9")
        .build()
        .unwrap();
    let err = camelmailer.server().ping().await.unwrap_err();
    assert!(matches!(err, Error::Network(_)));
}

#[tokio::test]
async fn forbidden_error_code_is_preserved() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/server/ping"))
        .respond_with(ResponseTemplate::new(403).set_body_json(error("Forbidden", "no access")))
        .mount(&server)
        .await;

    let err = client(&server).server().ping().await.unwrap_err();
    assert_eq!(err.code(), Some("Forbidden"));
    assert!(!err.is_unauthorized());
    assert!(!err.is_not_found());
}
