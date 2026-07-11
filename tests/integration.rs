//! Live integration test against a real CamelMailer instance.
//!
//! Skipped unless `CAMELMAILER_API_KEY` is set. NOT run in CI.
//!
//! ```sh
//! CAMELMAILER_API_KEY=cm_… \
//! CAMELMAILER_BASE_URL=https://mail.example.com \  # optional
//! CAMELMAILER_FROM=billing@acme.com \              # optional, enables the send test
//! CAMELMAILER_TO=you@example.com \                 # optional, enables the send test
//! cargo test --test integration -- --nocapture
//! ```

use camelmailer_rs::{CamelMailer, ListMessagesParams, SendEmailRequest, StatsParams};

fn live_client() -> Option<CamelMailer> {
    let api_key = std::env::var("CAMELMAILER_API_KEY").ok()?;
    let mut builder = CamelMailer::builder(api_key);
    if let Ok(base_url) = std::env::var("CAMELMAILER_BASE_URL") {
        builder = builder.base_url(base_url);
    }
    Some(builder.build().expect("valid configuration"))
}

#[tokio::test]
async fn live_roundtrip() {
    let Some(client) = live_client() else {
        eprintln!("skipping: CAMELMAILER_API_KEY is not set");
        return;
    };

    // 1. The key must be valid.
    let ping = client.server().ping().await.expect("ping");
    assert!(ping.pong);
    eprintln!("authenticated against server {:?}", ping.server);

    // 2. Reading resources must work.
    let stats = client.stats().get(StatsParams::new()).await.expect("stats");
    eprintln!("stats: {} total messages", stats.total);
    let templates = client.templates().list().await.expect("templates");
    eprintln!("templates: {}", templates.len());
    let streams = client.streams().list().await.expect("streams");
    eprintln!("streams: {}", streams.len());

    // 3. Optionally, send a real message and read it back.
    let (Ok(from), Ok(to)) = (
        std::env::var("CAMELMAILER_FROM"),
        std::env::var("CAMELMAILER_TO"),
    ) else {
        eprintln!("skipping send: CAMELMAILER_FROM / CAMELMAILER_TO not set");
        return;
    };
    let result = client
        .emails()
        .send(
            SendEmailRequest::builder()
                .from(from)
                .to(to)
                .subject("camelmailer-rs integration test")
                .text_body("Hello from the Rust SDK integration test.")
                .tag("sdk-integration-test")
                .build(),
        )
        .await
        .expect("send");
    let id = result.message_id.expect("queued message id");
    eprintln!("queued message {id}");

    let detail = client.emails().get(id).await.expect("message readback");
    assert_eq!(detail.message.id, id);

    let list = client
        .emails()
        .list(
            ListMessagesParams::new()
                .tag("sdk-integration-test")
                .per_page(5),
        )
        .await
        .expect("list");
    assert!(list.messages.iter().any(|m| m.id == id));
}
