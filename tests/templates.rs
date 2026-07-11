//! Wiremock tests for the templates service.

mod common;

use camelmailer_rs::TemplateFields;
use common::{client, error, success, API_KEY};
use serde_json::json;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn welcome_template() -> serde_json::Value {
    json!({
        "id": 3,
        "uuid": "7d3f7f60-0000-0000-0000-000000000000",
        "name": "Welcome",
        "permalink": "welcome",
        "subject": "Welcome, {{ name }}!",
        "html_body": "<p>Hi {{ name }}</p>",
        "text_body": "Hi {{ name }}",
        "archived": false,
    })
}

#[tokio::test]
async fn list_unwraps_templates() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/server/templates"))
        .and(header("X-Server-API-Key", API_KEY))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
            "templates": [welcome_template()],
        }))))
        .mount(&server)
        .await;

    let templates = client(&server).templates().list().await.unwrap();
    assert_eq!(templates.len(), 1);
    assert_eq!(templates[0].permalink, "welcome");
    assert!(!templates[0].archived);
}

#[tokio::test]
async fn create_posts_fields_and_returns_template() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/server/templates"))
        .and(body_json(json!({
            "name": "Welcome",
            "subject": "Welcome, {{ name }}!",
            "html_body": "<p>Hi {{ name }}</p>",
        })))
        .respond_with(ResponseTemplate::new(201).set_body_json(success(json!({
            "template": welcome_template(),
        }))))
        .expect(1)
        .mount(&server)
        .await;

    let template = client(&server)
        .templates()
        .create(
            TemplateFields::new("Welcome")
                .subject("Welcome, {{ name }}!")
                .html_body("<p>Hi {{ name }}</p>"),
        )
        .await
        .unwrap();
    assert_eq!(template.id, 3);
    assert_eq!(template.name, "Welcome");
}

#[tokio::test]
async fn get_fetches_by_permalink() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/server/templates/welcome"))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
            "template": welcome_template(),
        }))))
        .mount(&server)
        .await;

    let template = client(&server).templates().get("welcome").await.unwrap();
    assert_eq!(template.subject.as_deref(), Some("Welcome, {{ name }}!"));
}

#[tokio::test]
async fn update_patches_only_set_fields() {
    let server = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/api/v2/server/templates/welcome"))
        .and(body_json(json!({ "subject": "Hello, {{ name }}!" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
            "template": welcome_template(),
        }))))
        .expect(1)
        .mount(&server)
        .await;

    client(&server)
        .templates()
        .update(
            "welcome",
            TemplateFields::update().subject("Hello, {{ name }}!"),
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn archive_posts_and_returns_template() {
    let server = MockServer::start().await;
    let mut archived = welcome_template();
    archived["archived"] = json!(true);
    Mock::given(method("POST"))
        .and(path("/api/v2/server/templates/welcome/archive"))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
            "template": archived,
        }))))
        .mount(&server)
        .await;

    let template = client(&server)
        .templates()
        .archive("welcome")
        .await
        .unwrap();
    assert!(template.archived);
}

#[tokio::test]
async fn render_posts_model_and_returns_rendered_fields() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/server/templates/welcome/render"))
        .and(body_json(json!({ "template_model": { "name": "Ada" } })))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
            "rendered": {
                "subject": "Welcome, Ada!",
                "html_body": "<p>Hi Ada</p>",
                "text_body": "Hi Ada",
            },
        }))))
        .expect(1)
        .mount(&server)
        .await;

    let rendered = client(&server)
        .templates()
        .render("welcome", json!({ "name": "Ada" }))
        .await
        .unwrap();
    assert_eq!(rendered.subject.as_deref(), Some("Welcome, Ada!"));
    assert_eq!(rendered.text_body.as_deref(), Some("Hi Ada"));
}

#[tokio::test]
async fn missing_template_is_not_found() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/server/templates/nope"))
        .respond_with(ResponseTemplate::new(404).set_body_json(error("NotFound", "not found")))
        .mount(&server)
        .await;

    let err = client(&server).templates().get("nope").await.unwrap_err();
    assert!(err.is_not_found());
    assert_eq!(err.status(), Some(404));
}
