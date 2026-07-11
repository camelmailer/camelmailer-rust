# camelmailer-rs

[![CI](https://github.com/camelmailer/camelmailer-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/camelmailer/camelmailer-rust/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/camelmailer-rs.svg)](https://crates.io/crates/camelmailer-rs)
[![docs.rs](https://img.shields.io/docsrs/camelmailer-rs)](https://docs.rs/camelmailer-rs)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Rust SDK for [CamelMailer](https://camelmailer.com) — the open-source transactional email platform. Async-first (reqwest + rustls, no OpenSSL), fully typed, MSRV 1.75.

## Install

```sh
cargo add camelmailer-rs tokio --features tokio/full
```

## Quickstart

```rust
use camelmailer_rs::{CamelMailer, SendEmailRequest};

#[tokio::main]
async fn main() -> Result<(), camelmailer_rs::Error> {
    let client = CamelMailer::new("cm_xxxxxxxx");
    let email = SendEmailRequest::builder()
        .from("billing@acme.com")
        .to("ada@example.com")
        .subject("Your receipt")
        .html_body("<h1>Thanks for your purchase!</h1>")
        .build();
    let result = client.emails().send(email).await?;
    println!("queued: {:?}", result.message_id);
    Ok(())
}
```

## Self-hosted? Set your base URL

The client talks to the CamelMailer cloud (`https://app.camelmailer.com`) by default:

```rust,no_run
let client = camelmailer_rs::CamelMailer::builder("cm_xxxxxxxx")
    .base_url("https://mail.example.com")
    .build()?;
# Ok::<(), camelmailer_rs::Error>(())
```

## Emails

```rust,no_run
# use camelmailer_rs::*; use serde_json::json;
# async fn run(client: CamelMailer) -> Result<()> {
// Send with attachments, named addresses, metadata, tags
let email = SendEmailRequest::builder()
    .from(Address::with_name("billing@acme.com", "Acme Billing"))
    .to("ada@example.com")
    .cc("grace@example.com")
    .subject("Invoice #42")
    .html_body("<p>See attachment.</p>")
    .attachment(Attachment::from_bytes("invoice.pdf", "application/pdf", b"%PDF..."))
    .tag("invoice")
    .metadata(json!({ "order_id": 42 }))
    .build();
let result = client.emails().send(email).await?;

// Read it back
let id = result.message_id.unwrap();
let detail = client.emails().get(id).await?;
let deliveries = client.emails().deliveries(id).await?;
let opens = client.emails().opens(id).await?;
let clicks = client.emails().clicks(id).await?;
let raw = client.emails().raw(id).await?.decode()?;

// List with filters
let page = client.emails()
    .list(ListMessagesParams::new().scope("outgoing").tag("invoice").per_page(50))
    .await?;
# Ok(()) }
```

### Batch sending

One HTTP call, one result entry per message — entries can fail individually:

```rust,no_run
# use camelmailer_rs::*;
# async fn run(client: CamelMailer) -> Result<()> {
let entries = client.emails().send_batch(vec![
    SendEmailRequest::builder().from("a@acme.com").to("one@example.com").subject("Hi").build(),
    SendEmailRequest::builder().from("a@acme.com").to("two@example.com").subject("Hi").build(),
]).await?;
for entry in &entries {
    match entry.as_result() {
        Ok(sent) => println!("queued {:?}", sent.message_id),
        Err(error) => eprintln!("rejected: {error}"),
    }
}
# Ok(()) }
```

## Templates

```rust,no_run
# use camelmailer_rs::*; use serde_json::json;
# async fn run(client: CamelMailer) -> Result<()> {
// Create + render
let template = client.templates().create(
    TemplateFields::new("Welcome")
        .subject("Welcome, {{ name }}!")
        .html_body("<p>Hi {{ name }} 👋</p>"),
).await?;
let preview = client.templates().render(&template.permalink, json!({ "name": "Ada" })).await?;

// Send with a stored template
let result = client.emails().send_with_template(
    SendTemplateRequest::builder("welcome")
        .from("hello@acme.com")
        .to("ada@example.com")
        .model(json!({ "name": "Ada", "product": "Acme" }))
        .build(),
).await?;
# Ok(()) }
```

## Streams, stats, bounces, DMARC

```rust,no_run
# use camelmailer_rs::*;
# async fn run(client: CamelMailer) -> Result<()> {
let streams = client.streams().list().await?;
let stream = client.streams().create(CreateStreamRequest::new("Billing")).await?;

let stats = client.stats().get(StatsParams::new().from("2026-07-01T00:00:00Z")).await?;
let queue = client.stats().deliveries().await?;

let bounces = client.bounces().list(ListBouncesParams::new().per_page(25)).await?;

let dmarc = client.dmarc().summary(DmarcParams::new().domain("acme.com")).await?;
println!("DMARC pass rate: {:.1}%", dmarc.pass_rate * 100.0);
# Ok(()) }
```

## Error handling

Every API error carries the stable error code from the response envelope (`Unauthorized`, `Forbidden`, `NotFound`, `ValidationError`, `ParameterMissing`, …):

```rust,no_run
# use camelmailer_rs::{CamelMailer, Error};
# async fn run(client: CamelMailer) -> Result<(), Box<dyn std::error::Error>> {
match client.emails().get(42).await {
    Ok(detail) => println!("{:?}", detail.message.status),
    Err(error) if error.is_not_found() => println!("no such message"),
    Err(Error::Api { code, message, status }) => {
        eprintln!("API error {code} (HTTP {status}): {message}")
    }
    Err(Error::Network(source)) => eprintln!("network trouble: {source}"),
    Err(other) => return Err(other.into()),
}
# Ok(()) }
```

## Blocking client

No async runtime? Enable the `blocking` feature:

```toml
camelmailer-rs = { version = "0.1", features = ["blocking"] }
```

```rust,no_run
# use camelmailer_rs::SendEmailRequest;
# fn run() -> Result<(), camelmailer_rs::Error> {
let client = camelmailer_rs::blocking::CamelMailer::new("cm_xxxxxxxx")?;
let result = client.emails().send(
    SendEmailRequest::builder()
        .from("billing@acme.com")
        .to("ada@example.com")
        .subject("Hi")
        .text_body("Hello!")
        .build(),
)?;
# Ok(()) }
```

## Docs

- API reference: <https://docs.rs/camelmailer-rs>
- CamelMailer docs: <https://camelmailer.com/docs>

## License

[MIT](LICENSE)
