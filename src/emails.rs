//! Send and inspect messages (`/api/v2/server/messages`).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{Error, Result};
use crate::http::Http;
use crate::types::{Address, Attachment, Headers, Pagination};

/// Service for sending and reading messages. Obtain via
/// [`CamelMailer::emails`](crate::CamelMailer::emails).
#[derive(Debug, Clone, Copy)]
pub struct Emails<'a> {
    pub(crate) http: &'a Http,
    /// Set by [`Emails::idempotent`]; applied to every send this service
    /// performs.
    pub(crate) idempotency_key: Option<&'a str>,
}

impl<'a> Emails<'a> {
    /// Make the sends on this service replayable.
    ///
    /// Returns a view of the service that attaches `key` as the
    /// `Idempotency-Key` header. Sending the same key with the same body
    /// returns the original result instead of queuing a second copy;
    /// the same key with a different body is refused with
    /// `InvalidIdempotentRequest` (HTTP 409) rather than silently
    /// ignored. Keys are scoped to the server and a completed result is
    /// kept for 24 hours.
    ///
    /// ```no_run
    /// # use camelmailer_rs::{CamelMailer, SendEmailRequest};
    /// # async fn run() -> Result<(), camelmailer_rs::Error> {
    /// # let client = CamelMailer::new("cm_xxxx");
    /// # let request = SendEmailRequest::builder().from("a@acme.com").to("b@example.com").build();
    /// client.emails().idempotent("order-4711").send(request).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn idempotent(mut self, key: &'a str) -> Self {
        self.idempotency_key = Some(key);
        self
    }
}

impl Emails<'_> {
    /// Send one message (`POST /api/v2/server/messages`).
    ///
    /// Queues one copy per recipient; the result carries a delivery
    /// token per recipient.
    pub async fn send(&self, request: SendEmailRequest) -> Result<SendResult> {
        self.http
            .post_idempotent(
                "/api/v2/server/messages",
                Some(&request),
                self.idempotency_key,
            )
            .await
    }

    /// Send the same content to every subscriber of a broadcast stream
    /// (`POST /api/v2/server/streams/{permalink}/send`).
    ///
    /// Either give a subject with a body, or a template permalink with an
    /// optional model. The result counts `queued` against `skipped`:
    /// recipients past the per-request cap of 1000 are skipped rather
    /// than queued, so a larger audience wants a campaign.
    pub async fn send_to_stream(
        &self,
        permalink: &str,
        request: SendToStreamRequest,
    ) -> Result<StreamSendResult> {
        self.http
            .post(
                &format!("/api/v2/server/streams/{permalink}/send"),
                Some(&request),
            )
            .await
    }

    /// Send a batch of messages in one call
    /// (`POST /api/v2/server/messages/batch`).
    ///
    /// Returns one entry per request, in order; individual entries can
    /// fail without failing the whole batch.
    pub async fn send_batch(
        &self,
        requests: impl IntoIterator<Item = SendEmailRequest>,
    ) -> Result<Vec<BatchEntry>> {
        let requests: Vec<SendEmailRequest> = requests.into_iter().collect();
        let response: BatchResponse = self
            .http
            .post_idempotent(
                "/api/v2/server/messages/batch",
                Some(&requests),
                self.idempotency_key,
            )
            .await?;
        Ok(response.messages)
    }

    /// Render a stored template and send it
    /// (`POST /api/v2/server/messages/with_template`).
    pub async fn send_with_template(&self, request: SendTemplateRequest) -> Result<SendResult> {
        self.http
            .post_idempotent(
                "/api/v2/server/messages/with_template",
                Some(&request),
                self.idempotency_key,
            )
            .await
    }

    /// Send a stored template to many recipients in one call
    /// (`POST /api/v2/server/messages/with_template/batch`).
    pub async fn send_with_template_batch(
        &self,
        requests: impl IntoIterator<Item = SendTemplateRequest>,
    ) -> Result<Vec<BatchEntry>> {
        let requests: Vec<SendTemplateRequest> = requests.into_iter().collect();
        let response: BatchResponse = self
            .http
            .post_idempotent(
                "/api/v2/server/messages/with_template/batch",
                Some(&requests),
                self.idempotency_key,
            )
            .await?;
        Ok(response.messages)
    }

    /// Fetch one message with its delivery attempts
    /// (`GET /api/v2/server/messages/{id}`).
    pub async fn get(&self, id: i64) -> Result<MessageDetail> {
        self.http
            .get(&format!("/api/v2/server/messages/{id}"), None::<&()>)
            .await
    }

    /// List messages, filtered and paginated
    /// (`GET /api/v2/server/messages`).
    pub async fn list(&self, params: ListMessagesParams) -> Result<MessageList> {
        self.http
            .get("/api/v2/server/messages", Some(&params))
            .await
    }

    /// Delivery attempts of a message
    /// (`GET /api/v2/server/messages/{id}/deliveries`).
    pub async fn deliveries(&self, id: i64) -> Result<Vec<Delivery>> {
        let response: DeliveriesResponse = self
            .http
            .get(
                &format!("/api/v2/server/messages/{id}/deliveries"),
                None::<&()>,
            )
            .await?;
        Ok(response.deliveries)
    }

    /// Open events of a message
    /// (`GET /api/v2/server/messages/{id}/opens`).
    pub async fn opens(&self, id: i64) -> Result<Vec<ActivityEvent>> {
        let response: OpensResponse = self
            .http
            .get(&format!("/api/v2/server/messages/{id}/opens"), None::<&()>)
            .await?;
        Ok(response.opens)
    }

    /// Click events of a message
    /// (`GET /api/v2/server/messages/{id}/clicks`).
    pub async fn clicks(&self, id: i64) -> Result<Vec<ActivityEvent>> {
        let response: ClicksResponse = self
            .http
            .get(&format!("/api/v2/server/messages/{id}/clicks"), None::<&()>)
            .await?;
        Ok(response.clicks)
    }

    /// Raw RFC 5322 source of a message, base64-encoded
    /// (`GET /api/v2/server/messages/{id}/raw`).
    pub async fn raw(&self, id: i64) -> Result<RawMessage> {
        self.http
            .get(&format!("/api/v2/server/messages/{id}/raw"), None::<&()>)
            .await
    }
}

// ---------------------------------------------------------------- requests

/// A message to send. Build with [`SendEmailRequest::builder`].
///
/// ```
/// use camelmailer_rs::SendEmailRequest;
///
/// let email = SendEmailRequest::builder()
///     .from("billing@acme.com")
///     .to("ada@example.com")
///     .subject("Your receipt")
///     .html_body("<h1>Thanks!</h1>")
///     .tag("receipt")
///     .build();
/// ```
#[derive(Debug, Clone, Default, Serialize)]
pub struct SendEmailRequest {
    /// Sender address; its domain must be a verified sending domain.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<Address>,
    /// Recipients.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub to: Vec<Address>,
    /// CC recipients.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cc: Vec<Address>,
    /// BCC recipients.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub bcc: Vec<Address>,
    /// Reply-To addresses.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub reply_to: Vec<Address>,
    /// Subject line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    /// HTML body.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html_body: Option<String>,
    /// Plain-text body.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_body: Option<String>,
    /// Extra message headers.
    #[serde(skip_serializing_if = "Headers::is_empty")]
    pub headers: Headers,
    /// File attachments.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<Attachment>,
    /// Free-form tag for filtering and stats.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    /// Arbitrary JSON metadata stored with the message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
    /// Message-stream permalink (defaults to the server's default stream).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<String>,
}

impl SendEmailRequest {
    /// Start building a send request.
    pub fn builder() -> SendEmailRequestBuilder {
        SendEmailRequestBuilder::default()
    }
}

/// Builder for [`SendEmailRequest`].
#[derive(Debug, Clone, Default)]
pub struct SendEmailRequestBuilder {
    request: SendEmailRequest,
}

impl SendEmailRequestBuilder {
    /// Sender address (`"a@b.com"` or `Address::with_name(…)`).
    pub fn from(mut self, address: impl Into<Address>) -> Self {
        self.request.from = Some(address.into());
        self
    }

    /// Add a recipient; call repeatedly for multiple recipients.
    pub fn to(mut self, address: impl Into<Address>) -> Self {
        self.request.to.push(address.into());
        self
    }

    /// Add a CC recipient.
    pub fn cc(mut self, address: impl Into<Address>) -> Self {
        self.request.cc.push(address.into());
        self
    }

    /// Add a BCC recipient.
    pub fn bcc(mut self, address: impl Into<Address>) -> Self {
        self.request.bcc.push(address.into());
        self
    }

    /// Add a Reply-To address.
    pub fn reply_to(mut self, address: impl Into<Address>) -> Self {
        self.request.reply_to.push(address.into());
        self
    }

    /// Subject line.
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.request.subject = Some(subject.into());
        self
    }

    /// HTML body.
    pub fn html_body(mut self, html: impl Into<String>) -> Self {
        self.request.html_body = Some(html.into());
        self
    }

    /// Plain-text body.
    pub fn text_body(mut self, text: impl Into<String>) -> Self {
        self.request.text_body = Some(text.into());
        self
    }

    /// Set one extra message header.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.request.headers.insert(name.into(), value.into());
        self
    }

    /// Attach a file.
    pub fn attachment(mut self, attachment: Attachment) -> Self {
        self.request.attachments.push(attachment);
        self
    }

    /// Free-form tag for filtering and stats.
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.request.tag = Some(tag.into());
        self
    }

    /// Arbitrary JSON metadata stored with the message.
    pub fn metadata(mut self, metadata: Value) -> Self {
        self.request.metadata = Some(metadata);
        self
    }

    /// Message-stream permalink.
    pub fn stream(mut self, stream: impl Into<String>) -> Self {
        self.request.stream = Some(stream.into());
        self
    }

    /// Finish building.
    pub fn build(self) -> SendEmailRequest {
        self.request
    }
}

/// A templated message to send. Build with [`SendTemplateRequest::builder`].
///
/// Fields set directly (e.g. `subject`) override the rendered ones.
///
/// ```
/// use camelmailer_rs::SendTemplateRequest;
/// use serde_json::json;
///
/// let email = SendTemplateRequest::builder("welcome")
///     .from("hello@acme.com")
///     .to("ada@example.com")
///     .model(json!({ "name": "Ada", "product": "Acme" }))
///     .build();
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct SendTemplateRequest {
    /// Template permalink.
    pub template: String,
    /// Variables the template is rendered against.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_model: Option<Value>,
    /// The message fields (recipients, overrides, …).
    #[serde(flatten)]
    pub message: SendEmailRequest,
}

impl SendTemplateRequest {
    /// Start building a templated send request for `template`
    /// (a template permalink).
    pub fn builder(template: impl Into<String>) -> SendTemplateRequestBuilder {
        SendTemplateRequestBuilder {
            template: template.into(),
            template_model: None,
            message: SendEmailRequest::builder(),
        }
    }
}

/// Builder for [`SendTemplateRequest`].
#[derive(Debug, Clone)]
pub struct SendTemplateRequestBuilder {
    template: String,
    template_model: Option<Value>,
    message: SendEmailRequestBuilder,
}

impl SendTemplateRequestBuilder {
    /// Variables the template is rendered against.
    pub fn model(mut self, model: Value) -> Self {
        self.template_model = Some(model);
        self
    }

    /// Sender address.
    pub fn from(mut self, address: impl Into<Address>) -> Self {
        self.message = self.message.from(address);
        self
    }

    /// Add a recipient.
    pub fn to(mut self, address: impl Into<Address>) -> Self {
        self.message = self.message.to(address);
        self
    }

    /// Add a CC recipient.
    pub fn cc(mut self, address: impl Into<Address>) -> Self {
        self.message = self.message.cc(address);
        self
    }

    /// Add a BCC recipient.
    pub fn bcc(mut self, address: impl Into<Address>) -> Self {
        self.message = self.message.bcc(address);
        self
    }

    /// Override the rendered subject.
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.message = self.message.subject(subject);
        self
    }

    /// Free-form tag for filtering and stats.
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.message = self.message.tag(tag);
        self
    }

    /// Arbitrary JSON metadata stored with the message.
    pub fn metadata(mut self, metadata: Value) -> Self {
        self.message = self.message.metadata(metadata);
        self
    }

    /// Message-stream permalink.
    pub fn stream(mut self, stream: impl Into<String>) -> Self {
        self.message = self.message.stream(stream);
        self
    }

    /// Finish building.
    pub fn build(self) -> SendTemplateRequest {
        SendTemplateRequest {
            template: self.template,
            template_model: self.template_model,
            message: self.message.build(),
        }
    }
}

/// Filters for [`Emails::list`]. All fields are optional.
///
/// ```
/// use camelmailer_rs::ListMessagesParams;
///
/// let params = ListMessagesParams::new()
///     .scope("outgoing")
///     .status("Sent")
///     .tag("receipt")
///     .page(2)
///     .per_page(50);
/// ```
#[derive(Debug, Clone, Default, Serialize)]
pub struct ListMessagesParams {
    /// Page number (1-based).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u64>,
    /// Entries per page (max 100).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_page: Option<u64>,
    /// `incoming` or `outgoing`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    /// Message status, e.g. `Sent`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Filter by tag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    /// Substring match on subject / addresses.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    /// Restrict to one message stream (by permalink).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<String>,
}

impl ListMessagesParams {
    /// No filters: first page, server defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Page number (1-based).
    pub fn page(mut self, page: u64) -> Self {
        self.page = Some(page);
        self
    }

    /// Entries per page (max 100).
    pub fn per_page(mut self, per_page: u64) -> Self {
        self.per_page = Some(per_page);
        self
    }

    /// `incoming` or `outgoing`.
    pub fn scope(mut self, scope: impl Into<String>) -> Self {
        self.scope = Some(scope.into());
        self
    }

    /// Message status, e.g. `Sent`.
    pub fn status(mut self, status: impl Into<String>) -> Self {
        self.status = Some(status.into());
        self
    }

    /// Filter by tag.
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = Some(tag.into());
        self
    }

    /// Substring match on subject / addresses.
    pub fn query(mut self, query: impl Into<String>) -> Self {
        self.query = Some(query.into());
        self
    }

    /// Restrict to one message stream (by permalink).
    pub fn stream(mut self, stream: impl Into<String>) -> Self {
        self.stream = Some(stream.into());
        self
    }
}

/// Fields for [`Emails::send_to_stream`].
///
/// Give either a subject with a body, or a template permalink with an
/// optional model.
#[derive(Debug, Clone, Default, Serialize)]
pub struct SendToStreamRequest {
    /// Sender address.
    pub from: String,
    /// Message subject.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    /// HTML part.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html_body: Option<String>,
    /// Plain-text part.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_body: Option<String>,
    /// Permalink of a stored template to render instead of the bodies.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    /// Values for the template's `{{ variables }}`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_model: Option<Value>,
}

impl SendToStreamRequest {
    /// A broadcast from the given address.
    pub fn new(from: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            ..Default::default()
        }
    }

    /// Message subject.
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    /// HTML part.
    pub fn html_body(mut self, html_body: impl Into<String>) -> Self {
        self.html_body = Some(html_body.into());
        self
    }

    /// Plain-text part.
    pub fn text_body(mut self, text_body: impl Into<String>) -> Self {
        self.text_body = Some(text_body.into());
        self
    }

    /// Render a stored template instead of the bodies above.
    pub fn template(mut self, permalink: impl Into<String>) -> Self {
        self.template = Some(permalink.into());
        self
    }

    /// Values for the template's `{{ variables }}`.
    pub fn template_model(mut self, model: Value) -> Self {
        self.template_model = Some(model);
        self
    }
}

// --------------------------------------------------------------- responses

/// How a broadcast to a stream was split.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct StreamSendResult {
    /// Recipients queued.
    pub queued: i64,
    /// Recipients past the per-request cap of 1000.
    pub skipped: i64,
}

/// Result of sending one message.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct SendResult {
    /// Id of the first queued copy (one copy is queued per recipient).
    pub message_id: Option<i64>,
    /// One entry per recipient.
    pub recipients: Vec<SendRecipient>,
}

/// Per-recipient outcome of a send.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct SendRecipient {
    /// The recipient address.
    pub rcpt_to: String,
    /// Id of the queued message copy.
    pub message_id: Option<i64>,
    /// Public token of the queued message copy.
    pub token: String,
    /// Queue status, e.g. `queued`.
    pub status: String,
}

#[derive(Debug, Deserialize)]
struct BatchResponse {
    #[serde(default)]
    messages: Vec<BatchEntry>,
}

#[derive(Debug, Deserialize)]
struct DeliveriesResponse {
    #[serde(default)]
    deliveries: Vec<Delivery>,
}

#[derive(Debug, Deserialize)]
struct OpensResponse {
    #[serde(default)]
    opens: Vec<ActivityEvent>,
}

#[derive(Debug, Deserialize)]
struct ClicksResponse {
    #[serde(default)]
    clicks: Vec<ActivityEvent>,
}

/// One entry of a batch send response: either a queued message or a
/// per-entry error.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum BatchEntry {
    /// The entry was queued.
    Success {
        /// The send result for this entry.
        data: SendResult,
    },
    /// The entry was rejected.
    Error {
        /// The error for this entry.
        error: BatchError,
    },
}

impl BatchEntry {
    /// This entry as a `Result`, mapping per-entry errors to
    /// [`Error::Api`] (with HTTP status `0`, since the batch call
    /// itself succeeded).
    pub fn as_result(&self) -> Result<&SendResult> {
        match self {
            BatchEntry::Success { data } => Ok(data),
            BatchEntry::Error { error } => Err(Error::Api {
                code: error.code.clone(),
                message: error.message.clone(),
                status: 0,
            }),
        }
    }

    /// `true` when this entry was queued.
    pub fn is_success(&self) -> bool {
        matches!(self, BatchEntry::Success { .. })
    }
}

/// A per-entry error inside a batch response.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct BatchError {
    /// Stable machine-readable error code.
    pub code: String,
    /// Human-readable description.
    pub message: String,
}

/// A stored message (outgoing or incoming).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Message {
    /// Numeric message id.
    pub id: i64,
    /// Public token.
    pub token: String,
    /// `incoming` or `outgoing`.
    pub scope: String,
    /// Recipient address.
    pub rcpt_to: String,
    /// Sender address.
    pub mail_from: Option<String>,
    /// Subject line.
    pub subject: Option<String>,
    /// The `Message-ID` header value.
    pub message_id: Option<String>,
    /// Tag set at send time.
    pub tag: Option<String>,
    /// Current status, e.g. `Sent`.
    pub status: Option<String>,
    /// Whether this message is a bounce.
    pub bounce: bool,
    /// Spam check verdict.
    pub spam_status: Option<String>,
    /// Spam score.
    pub spam_score: Option<f64>,
    /// Whether the message is held.
    pub held: bool,
    /// Whether a threat was detected.
    pub threat: bool,
    /// Size in bytes.
    pub size: Option<i64>,
    /// Metadata set at send time.
    pub metadata: Option<Value>,
    /// Id of the message stream.
    pub stream_id: Option<i64>,
    /// Whether a hold was bypassed.
    pub bypassed: bool,
    /// Creation time (RFC 3339).
    pub created_at: String,
}

/// One message plus its delivery attempts.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct MessageDetail {
    /// The message.
    pub message: Message,
    /// Its delivery attempts.
    pub deliveries: Vec<Delivery>,
}

/// A page of messages.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct MessageList {
    /// The messages on this page.
    pub messages: Vec<Message>,
    /// Pagination info.
    pub pagination: Pagination,
}

/// One delivery attempt of a message.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Delivery {
    /// Numeric delivery id.
    pub id: i64,
    /// Status, e.g. `Sent`, `SoftFail`.
    pub status: String,
    /// Short human-readable detail.
    pub details: Option<String>,
    /// Raw SMTP transcript / output.
    pub output: Option<String>,
    /// Whether TLS was used.
    pub sent_with_ssl: bool,
    /// Attempt time (RFC 3339).
    pub created_at: String,
}

/// An open or click event.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ActivityEvent {
    /// Client IP address.
    pub ip_address: Option<String>,
    /// Client user agent.
    pub user_agent: Option<String>,
    /// Clicked URL (click events only).
    pub url: Option<String>,
    /// Event time (RFC 3339).
    pub created_at: String,
}

/// The raw RFC 5322 source of a message.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct RawMessage {
    /// Base64-encoded message source.
    pub raw_message: String,
}

impl RawMessage {
    /// Decode the base64 source into bytes.
    pub fn decode(&self) -> Result<Vec<u8>> {
        use base64::Engine as _;
        base64::engine::general_purpose::STANDARD
            .decode(&self.raw_message)
            .map_err(|error| Error::Decode(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn send_request_serializes_only_set_fields() {
        let request = SendEmailRequest::builder()
            .from("billing@acme.com")
            .to("ada@example.com")
            .subject("Your receipt")
            .text_body("Thanks!")
            .build();
        let value = serde_json::to_value(&request).unwrap();
        assert_eq!(
            value,
            json!({
                "from": "billing@acme.com",
                "to": ["ada@example.com"],
                "subject": "Your receipt",
                "text_body": "Thanks!",
            })
        );
    }

    #[test]
    fn template_request_flattens_message_fields() {
        let request = SendTemplateRequest::builder("welcome")
            .from("hello@acme.com")
            .to("ada@example.com")
            .model(json!({ "name": "Ada" }))
            .build();
        let value = serde_json::to_value(&request).unwrap();
        assert_eq!(
            value,
            json!({
                "template": "welcome",
                "template_model": { "name": "Ada" },
                "from": "hello@acme.com",
                "to": ["ada@example.com"],
            })
        );
    }

    #[test]
    fn batch_entry_parses_success_and_error() {
        let entries: Vec<BatchEntry> = serde_json::from_value(json!([
            { "status": "success", "data": { "message_id": 1, "recipients": [] } },
            { "status": "error", "error": { "code": "ValidationError", "message": "nope" } },
        ]))
        .unwrap();
        assert!(entries[0].is_success());
        assert!(!entries[1].is_success());
        let error = entries[1].as_result().unwrap_err();
        assert_eq!(error.code(), Some("ValidationError"));
    }

    #[test]
    fn raw_message_decodes() {
        let raw = RawMessage {
            raw_message: "aGVsbG8=".into(),
        };
        assert_eq!(raw.decode().unwrap(), b"hello");
        let bad = RawMessage {
            raw_message: "!!".into(),
        };
        assert!(matches!(bad.decode(), Err(Error::Decode(_))));
    }
}
