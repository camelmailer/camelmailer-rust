//! Opt-in subscribers of a broadcast stream
//! (`/api/v2/server/streams/{permalink}/subscribers`).

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::http::Http;

/// Service for stream subscribers. Obtain via
/// [`CamelMailer::subscribers`](crate::CamelMailer::subscribers).
///
/// A broadcast send to an address that is not subscribed is refused, so
/// this list is the audience.
#[derive(Debug, Clone, Copy)]
pub struct Subscribers<'a> {
    pub(crate) http: &'a Http,
}

impl Subscribers<'_> {
    /// The stream's subscribers, subscribed and unsubscribed alike
    /// (`GET /api/v2/server/streams/{permalink}/subscribers`).
    pub async fn list(&self, permalink: &str) -> Result<Vec<Subscriber>> {
        let response: SubscribersResponse = self.http.get(&base(permalink), None::<&()>).await?;
        Ok(response.subscribers)
    }

    /// Add or update one subscriber
    /// (`POST /api/v2/server/streams/{permalink}/subscribers`).
    ///
    /// Upserts by address, so calling it twice is safe.
    pub async fn add(&self, permalink: &str, subscriber: AddSubscriber) -> Result<Subscriber> {
        let response: SubscriberResponse =
            self.http.post(&base(permalink), Some(&subscriber)).await?;
        Ok(response.subscriber)
    }

    /// Add many addresses at once, all as subscribed
    /// (`POST /api/v2/server/streams/{permalink}/subscribers/import`).
    ///
    /// Blanks and duplicates within the request are skipped, so `added`
    /// can be lower than the number of addresses passed.
    pub async fn import(
        &self,
        permalink: &str,
        addresses: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<ImportResult> {
        let body = ImportRequest {
            addresses: addresses.into_iter().map(Into::into).collect(),
        };
        self.http
            .post(&format!("{}/import", base(permalink)), Some(&body))
            .await
    }

    /// Record a spam complaint against an address
    /// (`POST /api/v2/server/streams/{permalink}/subscribers/{address}/complaint`).
    ///
    /// Writes a stream-scoped suppression and flips the subscription to
    /// `unsubscribed`. Idempotent, so a feedback loop can replay it.
    pub async fn complaint(&self, permalink: &str, address: &str) -> Result<Subscriber> {
        let response: SubscriberResponse = self
            .http
            .post(
                &format!("{}/{}/complaint", base(permalink), encode(address)),
                None::<&()>,
            )
            .await?;
        Ok(response.subscriber)
    }

    /// Remove a subscriber from the stream entirely
    /// (`DELETE /api/v2/server/streams/{permalink}/subscribers/{address}`).
    pub async fn remove(&self, permalink: &str, address: &str) -> Result<DeleteResult> {
        self.http
            .delete(&format!("{}/{}", base(permalink), encode(address)))
            .await
    }
}

/// The subscriber collection path of one stream.
fn base(permalink: &str) -> String {
    format!("/api/v2/server/streams/{permalink}/subscribers")
}

/// Percent-encode an address for a path segment.
///
/// The plus has to survive, or a different address is addressed: a bare
/// `+` in a path is a literal plus to some servers and a space to
/// others, so it is escaped explicitly.
fn encode(address: &str) -> String {
    let mut out = String::with_capacity(address.len());
    for byte in address.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// Fields for [`Subscribers::add`].
#[derive(Debug, Clone, Default, Serialize)]
pub struct AddSubscriber {
    /// The email address.
    pub address: String,
    /// `subscribed` (default) or `unsubscribed`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

impl AddSubscriber {
    /// A subscription for the given address.
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            address: address.into(),
            ..Default::default()
        }
    }

    /// `subscribed` or `unsubscribed`.
    pub fn status(mut self, status: impl Into<String>) -> Self {
        self.status = Some(status.into());
        self
    }
}

#[derive(Debug, Serialize)]
struct ImportRequest {
    addresses: Vec<String>,
}

/// What an import wrote.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ImportResult {
    /// Subscriptions written.
    pub added: i64,
    /// The stream's subscriber count after the import.
    pub total: i64,
}

/// What a delete removed.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct DeleteResult {
    /// Whether the row was removed.
    pub deleted: bool,
}

/// One address on a broadcast stream.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Subscriber {
    /// Numeric id.
    pub id: i64,
    /// The email address.
    pub address: String,
    /// `subscribed` or `unsubscribed`.
    pub status: String,
    /// When the subscription row was created.
    pub created_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SubscribersResponse {
    #[serde(default)]
    subscribers: Vec<Subscriber>,
}

#[derive(Debug, Deserialize)]
struct SubscriberResponse {
    subscriber: Subscriber,
}
