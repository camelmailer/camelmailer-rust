//! Manual end-to-end check against a running instance. Not part of CI.
//!
//!   CAMELMAILER_API_KEY=… CAMELMAILER_BASE_URL=http://localhost:5001 \
//!     cargo run --all-features --example live_check

use std::time::{SystemTime, UNIX_EPOCH};

use camelmailer_rs::{
    AddSubscriber, CamelMailer, CreateAndSendCampaign, CreateDraftCampaign, CreateStreamRequest,
    Error, LayoutFields, ListInboundParams, ListLogsParams, SendEmailRequest, SendTemplateRequest,
    SendToStreamRequest, UpdateCampaign,
};
use serde_json::json;

struct Tally {
    ok: u32,
    bad: u32,
}

impl Tally {
    fn step<T>(&mut self, label: &str, result: Result<T, Error>, show: impl Fn(&T) -> String) {
        match result {
            Ok(value) => {
                println!("  ok   {label}  -> {}", show(&value));
                self.ok += 1;
            }
            Err(err) => {
                println!("  FAIL {label}  -> {err}");
                self.bad += 1;
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("CAMELMAILER_API_KEY")?;
    let base_url = std::env::var("CAMELMAILER_BASE_URL")?;
    let client = CamelMailer::builder(&api_key).base_url(&base_url).build()?;
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let stream = format!("rs-bc-{stamp}");
    let layout = format!("rs-layout-{stamp}");
    let mut t = Tally { ok: 0, bad: 0 };

    println!("layouts");
    t.step(
        "create",
        client
            .layouts()
            .create(
                LayoutFields::new()
                    .name(format!("Rs {stamp}"))
                    .permalink(&layout)
                    .html_wrapper("<html><body>{{{ content }}}</body></html>"),
            )
            .await,
        |l| l.permalink.clone(),
    );
    t.step("list", client.layouts().list().await, |l| {
        format!("{} layouts", l.len())
    });
    t.step("get", client.layouts().get(&layout).await, |l| {
        l.name.clone()
    });
    t.step(
        "update",
        client
            .layouts()
            .update(&layout, LayoutFields::new().name("Renamed"))
            .await,
        |l| l.name.clone(),
    );
    t.step(
        "upload_logo",
        client
            .layouts()
            .upload_logo(
                &layout,
                "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==",
            )
            .await,
        |l| l.url.clone(),
    );

    println!("\nstreams + subscribers");
    t.step(
        "create stream",
        client
            .streams()
            .create(
                CreateStreamRequest::new(format!("Rs {stamp}"))
                    .permalink(&stream)
                    .stream_type("broadcast"),
            )
            .await,
        |s| s.permalink.clone(),
    );
    let plus = format!("ada+{stamp}@example.test");
    t.step(
        "add",
        client
            .subscribers()
            .add(&stream, AddSubscriber::new(&plus).name("Ada"))
            .await,
        |s| s.status.clone(),
    );
    t.step(
        "import",
        client
            .subscribers()
            .import(
                &stream,
                [
                    "grace@example.test",
                    "alan@example.test",
                    "grace@example.test",
                    "",
                ],
            )
            .await,
        |r| format!("added={} total={}", r.added, r.total),
    );
    t.step("list", client.subscribers().list(&stream).await, |s| {
        format!("{} subscribers", s.len())
    });
    t.step(
        "complaint",
        client
            .subscribers()
            .complaint(&stream, "alan@example.test")
            .await,
        |s| s.status.clone(),
    );
    t.step(
        "remove (plus address)",
        client.subscribers().remove(&stream, &plus).await,
        |r| format!("deleted={}", r.deleted),
    );

    println!("\ncampaigns");
    let draft = client
        .campaigns()
        .create_draft(
            CreateDraftCampaign::new(&stream, "news@acmedev.io")
                .name("Rs draft")
                .subject("D")
                .text_body("d"),
        )
        .await;
    let campaign_id = draft.as_ref().map(|c| c.id).unwrap_or_default();
    t.step("create_draft", draft, |c| {
        format!("id={} status={}", c.id, c.status)
    });
    t.step(
        "create_draft scheduled",
        client
            .campaigns()
            .create_draft(
                CreateDraftCampaign::new(&stream, "news@acmedev.io")
                    .name("Rs sched")
                    .scheduled_at("2027-01-01T09:00:00Z"),
            )
            .await,
        |c| c.status.clone(),
    );
    t.step(
        "create_and_send",
        client
            .campaigns()
            .create_and_send(
                &stream,
                CreateAndSendCampaign::new("Rs now")
                    .from("news@acmedev.io")
                    .subject("N")
                    .text_body("n"),
            )
            .await,
        |c| c.status.clone(),
    );
    t.step("list", client.campaigns().list().await, |c| {
        format!("{} campaigns", c.len())
    });
    t.step(
        "list_for_stream",
        client.campaigns().list_for_stream(&stream).await,
        |c| format!("{} campaigns", c.len()),
    );
    t.step("get", client.campaigns().get(campaign_id).await, |d| {
        format!("total={} sent={}", d.stats.total, d.stats.sent)
    });
    t.step(
        "get_for_stream",
        client
            .campaigns()
            .get_for_stream(&stream, campaign_id)
            .await,
        |d| d.campaign.name.clone().unwrap_or_default(),
    );
    t.step(
        "update (schedule)",
        client
            .campaigns()
            .update(
                campaign_id,
                UpdateCampaign::new().scheduled_at("2027-01-01T09:00:00Z"),
            )
            .await,
        |c| c.status.clone(),
    );
    t.step(
        "update (clear_schedule)",
        client
            .campaigns()
            .update(campaign_id, UpdateCampaign::new().clear_schedule())
            .await,
        |c| c.status.clone(),
    );
    t.step(
        "cancel",
        client.campaigns().cancel(campaign_id).await,
        |c| c.status.clone(),
    );

    println!("\nemails");
    let message = |to: &str| {
        SendEmailRequest::builder()
            .from("test@acmedev.io")
            .to(to)
            .subject(format!("Rs {stamp}"))
            .text_body("hi")
            .build()
    };
    t.step(
        "send",
        client.emails().send(message("live@example.test")).await,
        |r| format!("{:?}", r.message_id),
    );
    t.step(
        "send_batch",
        client
            .emails()
            .send_batch([message("one@example.test"), message("two@example.test")])
            .await,
        |entries| {
            entries
                .iter()
                .map(|e| match e.as_result() {
                    Ok(result) => format!("{:?}", result.message_id),
                    Err(err) => format!("err:{err}"),
                })
                .collect::<Vec<_>>()
                .join(" ")
        },
    );
    let key = format!("rs-live-{stamp}");
    let first = client
        .emails()
        .idempotent(&key)
        .send(message("idem@example.test"))
        .await;
    let first_id = first.as_ref().ok().and_then(|r| r.message_id);
    t.step("send with key", first, |r| format!("{:?}", r.message_id));
    t.step(
        "same key replays",
        client
            .emails()
            .idempotent(&key)
            .send(message("idem@example.test"))
            .await
            .and_then(|r| {
                if r.message_id == first_id {
                    Ok(r)
                } else {
                    Err(Error::Api {
                        code: "Replay".into(),
                        message: format!("got {:?}, want {first_id:?}", r.message_id),
                        status: 0,
                    })
                }
            }),
        |r| format!("{:?} (same)", r.message_id),
    );
    let other = SendEmailRequest::builder()
        .from("test@acmedev.io")
        .to("idem@example.test")
        .subject("Different")
        .text_body("twice")
        .build();
    match client.emails().idempotent(&key).send(other).await {
        Err(Error::Api { code, status, .. }) if code == "InvalidIdempotentRequest" => {
            println!("  ok   same key + other body is refused  -> {code} {status}");
            t.ok += 1;
        }
        other => {
            println!("  FAIL same key + other body is refused  -> {other:?}");
            t.bad += 1;
        }
    }
    t.step(
        "send_with_template + key",
        client
            .emails()
            .idempotent(&format!("{key}-tpl"))
            .send_with_template(
                SendTemplateRequest::builder("welcome")
                    .from("test@acmedev.io")
                    .to("tpl@example.test")
                    .model(json!({ "name": "Ada" }))
                    .build(),
            )
            .await,
        |r| format!("{:?}", r.message_id),
    );
    t.step(
        "send_to_stream",
        client
            .emails()
            .send_to_stream(
                &stream,
                SendToStreamRequest::new("news@acmedev.io")
                    .subject("Broadcast")
                    .text_body("hello"),
            )
            .await,
        |r| format!("queued={} skipped={}", r.queued, r.skipped),
    );

    println!("\ninbound + logs");
    t.step(
        "inbound list",
        client
            .inbound()
            .list(ListInboundParams::new().per_page(5))
            .await,
        |p| format!("{} inbound of {}", p.inbound.len(), p.pagination.total),
    );
    t.step(
        "logs list",
        client.logs().list(ListLogsParams::new().per_page(5)).await,
        |p| format!("{} requests", p.requests.len()),
    );
    t.step("tags", client.logs().tags().await, |t| {
        format!("{} tags", t.len())
    });

    println!("\nblocking mirror");
    let blocking_stream = stream.clone();
    let blocking_key = api_key.clone();
    let blocking_url = base_url.clone();
    let blocking = tokio::task::spawn_blocking(move || -> Result<(String, usize), Error> {
        let client = camelmailer_rs::blocking::CamelMailer::builder(blocking_key)
            .base_url(blocking_url)
            .build()?;
        let campaign = client.campaigns().create_draft(
            CreateDraftCampaign::new(&blocking_stream, "news@acmedev.io").name("Rs blocking draft"),
        )?;
        let tags = client.logs().tags()?;
        Ok((campaign.status, tags.len()))
    })
    .await
    .expect("blocking task");
    t.step("create_draft + tags", blocking, |(status, tags)| {
        format!("status={status} {tags} tags")
    });

    println!("\ncleanup");
    t.step(
        "delete layout",
        client.layouts().delete(&layout).await,
        |r| format!("deleted={}", r.deleted),
    );
    t.step(
        "archive stream",
        client.streams().archive(&stream).await,
        |s| format!("archived={}", s.archived),
    );

    println!("\n{} ok, {} failed", t.ok, t.bad);
    if t.bad > 0 {
        std::process::exit(1);
    }
    Ok(())
}
