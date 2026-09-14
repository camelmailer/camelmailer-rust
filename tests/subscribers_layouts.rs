//! Wiremock tests for the subscribers and layouts services.

mod common;

use camelmailer_rs::{AddSubscriber, LayoutFields};
use common::{client, error, success};
use serde_json::json;
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn subscribers_list_unwraps() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/server/streams/product-news/subscribers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
            "subscribers": [{
                "id": 1, "address": "ada@example.com", "status": "subscribed",
                "created_at": "2026-09-01T10:00:00Z",
            }],
        }))))
        .mount(&server)
        .await;

    let subscribers = client(&server)
        .subscribers()
        .list("product-news")
        .await
        .unwrap();
    assert_eq!(subscribers.len(), 1);
    assert_eq!(subscribers[0].status, "subscribed");
}

#[tokio::test]
async fn subscribers_add_upserts_by_address() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/server/streams/product-news/subscribers"))
        .and(body_json(
            json!({ "address": "ada@example.com", "status": "subscribed" }),
        ))
        .respond_with(ResponseTemplate::new(201).set_body_json(success(json!({
            "subscriber": { "id": 1, "address": "ada@example.com", "status": "subscribed" },
        }))))
        .mount(&server)
        .await;

    let subscriber = client(&server)
        .subscribers()
        .add(
            "product-news",
            AddSubscriber::new("ada@example.com").status("subscribed"),
        )
        .await
        .unwrap();
    assert_eq!(subscriber.address, "ada@example.com");
}

#[tokio::test]
async fn subscribers_import_sends_an_addresses_array() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/api/v2/server/streams/product-news/subscribers/import",
        ))
        .and(body_json(json!({
            "addresses": ["ada@example.com", "grace@example.com"],
        })))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(success(json!({ "added": 2, "total": 2 }))),
        )
        .mount(&server)
        .await;

    let result = client(&server)
        .subscribers()
        .import("product-news", ["ada@example.com", "grace@example.com"])
        .await
        .unwrap();
    assert_eq!(result.added, 2);
}

#[tokio::test]
async fn subscribers_remove_escapes_the_address() {
    let server = MockServer::start().await;
    // The plus has to survive the path, or a different address is removed.
    Mock::given(method("DELETE"))
        .and(path(
            "/api/v2/server/streams/product-news/subscribers/ada%2Bnews%40example.com",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({ "deleted": true }))))
        .mount(&server)
        .await;

    let result = client(&server)
        .subscribers()
        .remove("product-news", "ada+news@example.com")
        .await
        .unwrap();
    assert!(result.deleted);
}

#[tokio::test]
async fn subscribers_complaint_unsubscribes() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/api/v2/server/streams/product-news/subscribers/ada%40example.com/complaint",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
            "subscriber": { "id": 1, "address": "ada@example.com", "status": "unsubscribed" },
        }))))
        .mount(&server)
        .await;

    let subscriber = client(&server)
        .subscribers()
        .complaint("product-news", "ada@example.com")
        .await
        .unwrap();
    assert_eq!(subscriber.status, "unsubscribed");
}

#[tokio::test]
async fn layouts_list_unwraps() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/server/layouts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
            "layouts": [{
                "id": 1, "uuid": "l-1", "name": "Default", "permalink": "default",
                "html_wrapper": "<html>{{{ content }}}</html>", "text_wrapper": "{{{ content }}}",
            }],
        }))))
        .mount(&server)
        .await;

    let layouts = client(&server).layouts().list().await.unwrap();
    assert_eq!(layouts[0].permalink, "default");
    assert_eq!(layouts[0].html_wrapper, "<html>{{{ content }}}</html>");
}

#[tokio::test]
async fn a_layout_without_a_text_wrapper_deserializes() {
    let server = MockServer::start().await;
    // A layout created with only an HTML wrapper comes back with an
    // explicit null for the text one, which a plain String would reject.
    Mock::given(method("GET"))
        .and(path("/api/v2/server/layouts/default"))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
            "layout": {
                "id": 1, "uuid": "l-1", "name": "Default", "permalink": "default",
                "html_wrapper": "<html>{{{ content }}}</html>", "text_wrapper": null,
            },
        }))))
        .mount(&server)
        .await;

    let layout = client(&server).layouts().get("default").await.unwrap();
    assert!(layout.text_wrapper.is_none());
}

#[tokio::test]
async fn layouts_create_sends_the_wrapper() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/server/layouts"))
        .and(body_json(json!({
            "name": "Default",
            "permalink": "default",
            "html_wrapper": "<html>{{{ content }}}</html>",
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(success(json!({
            "layout": { "id": 1, "name": "Default", "permalink": "default" },
        }))))
        .mount(&server)
        .await;

    let layout = client(&server)
        .layouts()
        .create(
            LayoutFields::new()
                .name("Default")
                .permalink("default")
                .html_wrapper("<html>{{{ content }}}</html>"),
        )
        .await
        .unwrap();
    assert_eq!(layout.name, "Default");
}

#[tokio::test]
async fn layouts_create_without_the_placeholder_is_refused() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/server/layouts"))
        .respond_with(ResponseTemplate::new(422).set_body_json(error(
            "ValidationError",
            "html_wrapper must contain {{{ content }}}",
        )))
        .mount(&server)
        .await;

    let err = client(&server)
        .layouts()
        .create(
            LayoutFields::new()
                .name("Broken")
                .html_wrapper("<html></html>"),
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        camelmailer_rs::Error::Api { ref code, .. } if code == "ValidationError"
    ));
}

#[tokio::test]
async fn layouts_delete_and_logo() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/api/v2/server/layouts/default"))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({ "deleted": true }))))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/v2/server/layouts/default/logo"))
        .and(body_json(
            json!({ "data_url": "data:image/png;base64,iVBORw0KGgo=" }),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
            "url": "https://app.camelmailer.com/assets/layouts/l-1/logo",
        }))))
        .mount(&server)
        .await;

    let client = client(&server);
    assert!(client.layouts().delete("default").await.unwrap().deleted);
    let logo = client
        .layouts()
        .upload_logo("default", "data:image/png;base64,iVBORw0KGgo=")
        .await
        .unwrap();
    // The API names this key `url`, not `logo_url`.
    assert_eq!(
        logo.url,
        "https://app.camelmailer.com/assets/layouts/l-1/logo"
    );
}
