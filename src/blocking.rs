//! Blocking client (enable the `blocking` feature).
//!
//! Wraps the async client in a private tokio runtime, mirroring the
//! full API surface with synchronous methods:
//!
//! ```no_run
//! use camelmailer_rs::blocking::CamelMailer;
//! use camelmailer_rs::SendEmailRequest;
//!
//! # fn run() -> Result<(), camelmailer_rs::Error> {
//! let client = CamelMailer::new("cm_xxxxxxxx")?;
//! let result = client.emails().send(
//!     SendEmailRequest::builder()
//!         .from("billing@acme.com")
//!         .to("ada@example.com")
//!         .subject("Your receipt")
//!         .text_body("Thanks!")
//!         .build(),
//! )?;
//! # Ok(())
//! # }
//! ```
//!
//! Do not use this client inside an async runtime — it blocks the
//! current thread.

use serde_json::Value;
use std::sync::Arc;

use crate::error::{Error, Result};
use crate::{
    AddSubscriber, BatchEntry, BounceList, Campaign, CampaignDetail, CreateAndSendCampaign,
    CreateDraftCampaign, CreateStreamRequest, DeleteResult, DeliveryStats, DmarcParams,
    DmarcReportDetail, DmarcReportList, DmarcSummary, ImportResult, InboundList, Layout,
    LayoutFields, LayoutLogo, ListBouncesParams, ListInboundParams, ListLogsParams,
    ListMessagesParams, LogList, Message, MessageDetail, MessageList, Ping, RawMessage,
    RenderedTemplate, RequeueResult, SendEmailRequest, SendResult, SendTemplateRequest,
    SendToStreamRequest, Stats, StatsParams, StreamSendResult, Subscriber, TagCount, Template,
    TemplateFields, UpdateCampaign, UpdateStreamRequest,
};

/// The blocking CamelMailer client. Mirrors [`crate::CamelMailer`].
///
/// Cheap to clone; clones share the runtime and connection pool.
#[derive(Debug, Clone)]
pub struct CamelMailer {
    inner: crate::CamelMailer,
    runtime: Arc<tokio::runtime::Runtime>,
}

impl CamelMailer {
    /// A blocking client for the CamelMailer cloud.
    pub fn new(api_key: impl Into<String>) -> Result<Self> {
        Self::from_async(crate::CamelMailer::new(api_key))
    }

    /// Start configuring a blocking client.
    pub fn builder(api_key: impl Into<String>) -> Builder {
        Builder {
            inner: crate::CamelMailer::builder(api_key),
        }
    }

    fn from_async(inner: crate::CamelMailer) -> Result<Self> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| Error::Config(format!("failed to start runtime: {error}")))?;
        Ok(Self {
            inner,
            runtime: Arc::new(runtime),
        })
    }

    /// The base URL this client talks to (no trailing slash).
    pub fn base_url(&self) -> &str {
        self.inner.base_url()
    }

    /// Send and inspect messages.
    pub fn emails(&self) -> Emails<'_> {
        Emails {
            client: self,
            idempotency_key: None,
        }
    }

    /// Manage and render templates.
    pub fn templates(&self) -> Templates<'_> {
        Templates { client: self }
    }

    /// Manage message streams.
    pub fn streams(&self) -> Streams<'_> {
        Streams { client: self }
    }

    /// Message and queue statistics.
    pub fn stats(&self) -> StatsService<'_> {
        StatsService { client: self }
    }

    /// Bounced messages.
    pub fn bounces(&self) -> Bounces<'_> {
        Bounces { client: self }
    }

    /// DMARC monitoring.
    pub fn dmarc(&self) -> Dmarc<'_> {
        Dmarc { client: self }
    }

    /// Broadcast campaigns.
    pub fn campaigns(&self) -> Campaigns<'_> {
        Campaigns { client: self }
    }

    /// Opt-in subscribers of a broadcast stream.
    pub fn subscribers(&self) -> Subscribers<'_> {
        Subscribers { client: self }
    }

    /// Template layouts.
    pub fn layouts(&self) -> Layouts<'_> {
        Layouts { client: self }
    }

    /// Inbound and held messages.
    pub fn inbound(&self) -> Inbound<'_> {
        Inbound { client: self }
    }

    /// The server's request log and tag index.
    pub fn logs(&self) -> Logs<'_> {
        Logs { client: self }
    }

    /// The authenticated server (show / ping).
    pub fn server(&self) -> Servers<'_> {
        Servers { client: self }
    }

    fn block_on<F: std::future::Future>(&self, future: F) -> F::Output {
        self.runtime.block_on(future)
    }
}

/// Builder for the blocking [`CamelMailer`].
#[derive(Debug)]
pub struct Builder {
    inner: crate::CamelMailerBuilder,
}

impl Builder {
    /// Base URL of the instance, e.g. `https://mail.example.com`.
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.inner = self.inner.base_url(base_url);
        self
    }

    /// Request timeout (no timeout by default).
    pub fn timeout(mut self, timeout: std::time::Duration) -> Self {
        self.inner = self.inner.timeout(timeout);
        self
    }

    /// Build the blocking client.
    pub fn build(self) -> Result<CamelMailer> {
        CamelMailer::from_async(self.inner.build()?)
    }
}

/// Blocking counterpart of [`crate::Emails`].
#[derive(Debug, Clone, Copy)]
pub struct Emails<'a> {
    client: &'a CamelMailer,
    idempotency_key: Option<&'a str>,
}

impl<'a> Emails<'a> {
    /// See [`crate::Emails::idempotent`].
    pub fn idempotent(mut self, key: &'a str) -> Self {
        self.idempotency_key = Some(key);
        self
    }

    /// The async service this one delegates to, carrying the key.
    fn inner(&self) -> crate::Emails<'_> {
        let emails = self.client.inner.emails();
        match self.idempotency_key {
            Some(key) => emails.idempotent(key),
            None => emails,
        }
    }
}

impl Emails<'_> {
    /// See [`crate::Emails::send`].
    pub fn send(&self, request: SendEmailRequest) -> Result<SendResult> {
        self.client.block_on(self.inner().send(request))
    }

    /// See [`crate::Emails::send_to_stream`].
    pub fn send_to_stream(
        &self,
        permalink: &str,
        request: SendToStreamRequest,
    ) -> Result<StreamSendResult> {
        self.client.block_on(
            self.client
                .inner
                .emails()
                .send_to_stream(permalink, request),
        )
    }

    /// See [`crate::Emails::send_batch`].
    pub fn send_batch(
        &self,
        requests: impl IntoIterator<Item = SendEmailRequest>,
    ) -> Result<Vec<BatchEntry>> {
        self.client.block_on(self.inner().send_batch(requests))
    }

    /// See [`crate::Emails::send_with_template`].
    pub fn send_with_template(&self, request: SendTemplateRequest) -> Result<SendResult> {
        self.client
            .block_on(self.inner().send_with_template(request))
    }

    /// See [`crate::Emails::send_with_template_batch`].
    pub fn send_with_template_batch(
        &self,
        requests: impl IntoIterator<Item = SendTemplateRequest>,
    ) -> Result<Vec<BatchEntry>> {
        self.client
            .block_on(self.inner().send_with_template_batch(requests))
    }

    /// See [`crate::Emails::get`].
    pub fn get(&self, id: i64) -> Result<MessageDetail> {
        self.client.block_on(self.client.inner.emails().get(id))
    }

    /// See [`crate::Emails::list`].
    pub fn list(&self, params: ListMessagesParams) -> Result<MessageList> {
        self.client
            .block_on(self.client.inner.emails().list(params))
    }

    /// See [`crate::Emails::deliveries`].
    pub fn deliveries(&self, id: i64) -> Result<Vec<crate::Delivery>> {
        self.client
            .block_on(self.client.inner.emails().deliveries(id))
    }

    /// See [`crate::Emails::opens`].
    pub fn opens(&self, id: i64) -> Result<Vec<crate::ActivityEvent>> {
        self.client.block_on(self.client.inner.emails().opens(id))
    }

    /// See [`crate::Emails::clicks`].
    pub fn clicks(&self, id: i64) -> Result<Vec<crate::ActivityEvent>> {
        self.client.block_on(self.client.inner.emails().clicks(id))
    }

    /// See [`crate::Emails::raw`].
    pub fn raw(&self, id: i64) -> Result<RawMessage> {
        self.client.block_on(self.client.inner.emails().raw(id))
    }
}

/// Blocking counterpart of [`crate::Templates`].
#[derive(Debug, Clone, Copy)]
pub struct Templates<'a> {
    client: &'a CamelMailer,
}

impl Templates<'_> {
    /// See [`crate::Templates::list`].
    pub fn list(&self) -> Result<Vec<Template>> {
        self.client.block_on(self.client.inner.templates().list())
    }

    /// See [`crate::Templates::create`].
    pub fn create(&self, template: TemplateFields) -> Result<Template> {
        self.client
            .block_on(self.client.inner.templates().create(template))
    }

    /// See [`crate::Templates::get`].
    pub fn get(&self, permalink: &str) -> Result<Template> {
        self.client
            .block_on(self.client.inner.templates().get(permalink))
    }

    /// See [`crate::Templates::update`].
    pub fn update(&self, permalink: &str, fields: TemplateFields) -> Result<Template> {
        self.client
            .block_on(self.client.inner.templates().update(permalink, fields))
    }

    /// See [`crate::Templates::archive`].
    pub fn archive(&self, permalink: &str) -> Result<Template> {
        self.client
            .block_on(self.client.inner.templates().archive(permalink))
    }

    /// See [`crate::Templates::render`].
    pub fn render(&self, permalink: &str, model: Value) -> Result<RenderedTemplate> {
        self.client
            .block_on(self.client.inner.templates().render(permalink, model))
    }
}

/// Blocking counterpart of [`crate::Streams`].
#[derive(Debug, Clone, Copy)]
pub struct Streams<'a> {
    client: &'a CamelMailer,
}

impl Streams<'_> {
    /// See [`crate::Streams::list`].
    pub fn list(&self) -> Result<Vec<crate::Stream>> {
        self.client.block_on(self.client.inner.streams().list())
    }

    /// See [`crate::Streams::create`].
    pub fn create(&self, stream: CreateStreamRequest) -> Result<crate::Stream> {
        self.client
            .block_on(self.client.inner.streams().create(stream))
    }

    /// See [`crate::Streams::get`].
    pub fn get(&self, permalink: &str) -> Result<crate::Stream> {
        self.client
            .block_on(self.client.inner.streams().get(permalink))
    }

    /// See [`crate::Streams::update`].
    pub fn update(&self, permalink: &str, fields: UpdateStreamRequest) -> Result<crate::Stream> {
        self.client
            .block_on(self.client.inner.streams().update(permalink, fields))
    }

    /// See [`crate::Streams::archive`].
    pub fn archive(&self, permalink: &str) -> Result<crate::Stream> {
        self.client
            .block_on(self.client.inner.streams().archive(permalink))
    }
}

/// Blocking counterpart of [`crate::StatsService`].
#[derive(Debug, Clone, Copy)]
pub struct StatsService<'a> {
    client: &'a CamelMailer,
}

impl StatsService<'_> {
    /// See [`crate::StatsService::get`].
    pub fn get(&self, params: StatsParams) -> Result<Stats> {
        self.client.block_on(self.client.inner.stats().get(params))
    }

    /// See [`crate::StatsService::deliveries`].
    pub fn deliveries(&self) -> Result<DeliveryStats> {
        self.client.block_on(self.client.inner.stats().deliveries())
    }
}

/// Blocking counterpart of [`crate::Bounces`].
#[derive(Debug, Clone, Copy)]
pub struct Bounces<'a> {
    client: &'a CamelMailer,
}

impl Bounces<'_> {
    /// See [`crate::Bounces::list`].
    pub fn list(&self, params: ListBouncesParams) -> Result<BounceList> {
        self.client
            .block_on(self.client.inner.bounces().list(params))
    }

    /// See [`crate::Bounces::get`].
    pub fn get(&self, id: i64) -> Result<Message> {
        self.client.block_on(self.client.inner.bounces().get(id))
    }
}

/// Blocking counterpart of [`crate::Dmarc`].
#[derive(Debug, Clone, Copy)]
pub struct Dmarc<'a> {
    client: &'a CamelMailer,
}

impl Dmarc<'_> {
    /// See [`crate::Dmarc::summary`].
    pub fn summary(&self, params: DmarcParams) -> Result<DmarcSummary> {
        self.client
            .block_on(self.client.inner.dmarc().summary(params))
    }

    /// See [`crate::Dmarc::reports`].
    pub fn reports(&self, params: DmarcParams) -> Result<DmarcReportList> {
        self.client
            .block_on(self.client.inner.dmarc().reports(params))
    }

    /// See [`crate::Dmarc::report`].
    pub fn report(&self, id: i64) -> Result<DmarcReportDetail> {
        self.client.block_on(self.client.inner.dmarc().report(id))
    }
}

/// Blocking counterpart of [`crate::Servers`].
#[derive(Debug, Clone, Copy)]
pub struct Servers<'a> {
    client: &'a CamelMailer,
}

impl Servers<'_> {
    /// See [`crate::Servers::get`].
    pub fn get(&self) -> Result<crate::Server> {
        self.client.block_on(self.client.inner.server().get())
    }

    /// See [`crate::Servers::ping`].
    pub fn ping(&self) -> Result<Ping> {
        self.client.block_on(self.client.inner.server().ping())
    }
}

/// Blocking counterpart of [`crate::Campaigns`].
#[derive(Debug, Clone, Copy)]
pub struct Campaigns<'a> {
    client: &'a CamelMailer,
}

impl Campaigns<'_> {
    /// See [`crate::Campaigns::list`].
    pub fn list(&self) -> Result<Vec<Campaign>> {
        self.client.block_on(self.client.inner.campaigns().list())
    }

    /// See [`crate::Campaigns::list_for_stream`].
    pub fn list_for_stream(&self, permalink: &str) -> Result<Vec<Campaign>> {
        self.client
            .block_on(self.client.inner.campaigns().list_for_stream(permalink))
    }

    /// See [`crate::Campaigns::get`].
    pub fn get(&self, id: i64) -> Result<CampaignDetail> {
        self.client.block_on(self.client.inner.campaigns().get(id))
    }

    /// See [`crate::Campaigns::get_for_stream`].
    pub fn get_for_stream(&self, permalink: &str, id: i64) -> Result<CampaignDetail> {
        self.client
            .block_on(self.client.inner.campaigns().get_for_stream(permalink, id))
    }

    /// See [`crate::Campaigns::create_draft`].
    pub fn create_draft(&self, campaign: CreateDraftCampaign) -> Result<Campaign> {
        self.client
            .block_on(self.client.inner.campaigns().create_draft(campaign))
    }

    /// See [`crate::Campaigns::create_and_send`].
    pub fn create_and_send(
        &self,
        permalink: &str,
        campaign: CreateAndSendCampaign,
    ) -> Result<Campaign> {
        self.client.block_on(
            self.client
                .inner
                .campaigns()
                .create_and_send(permalink, campaign),
        )
    }

    /// See [`crate::Campaigns::update`].
    pub fn update(&self, id: i64, fields: UpdateCampaign) -> Result<Campaign> {
        self.client
            .block_on(self.client.inner.campaigns().update(id, fields))
    }

    /// See [`crate::Campaigns::send`].
    pub fn send(&self, id: i64) -> Result<Campaign> {
        self.client.block_on(self.client.inner.campaigns().send(id))
    }

    /// See [`crate::Campaigns::cancel`].
    pub fn cancel(&self, id: i64) -> Result<Campaign> {
        self.client
            .block_on(self.client.inner.campaigns().cancel(id))
    }
}

/// Blocking counterpart of [`crate::Subscribers`].
#[derive(Debug, Clone, Copy)]
pub struct Subscribers<'a> {
    client: &'a CamelMailer,
}

impl Subscribers<'_> {
    /// See [`crate::Subscribers::list`].
    pub fn list(&self, permalink: &str) -> Result<Vec<Subscriber>> {
        self.client
            .block_on(self.client.inner.subscribers().list(permalink))
    }

    /// See [`crate::Subscribers::add`].
    pub fn add(&self, permalink: &str, subscriber: AddSubscriber) -> Result<Subscriber> {
        self.client
            .block_on(self.client.inner.subscribers().add(permalink, subscriber))
    }

    /// See [`crate::Subscribers::import`].
    pub fn import(
        &self,
        permalink: &str,
        addresses: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<ImportResult> {
        self.client
            .block_on(self.client.inner.subscribers().import(permalink, addresses))
    }

    /// See [`crate::Subscribers::complaint`].
    pub fn complaint(&self, permalink: &str, address: &str) -> Result<Subscriber> {
        self.client.block_on(
            self.client
                .inner
                .subscribers()
                .complaint(permalink, address),
        )
    }

    /// See [`crate::Subscribers::remove`].
    pub fn remove(&self, permalink: &str, address: &str) -> Result<DeleteResult> {
        self.client
            .block_on(self.client.inner.subscribers().remove(permalink, address))
    }
}

/// Blocking counterpart of [`crate::Layouts`].
#[derive(Debug, Clone, Copy)]
pub struct Layouts<'a> {
    client: &'a CamelMailer,
}

impl Layouts<'_> {
    /// See [`crate::Layouts::list`].
    pub fn list(&self) -> Result<Vec<Layout>> {
        self.client.block_on(self.client.inner.layouts().list())
    }

    /// See [`crate::Layouts::create`].
    pub fn create(&self, layout: LayoutFields) -> Result<Layout> {
        self.client
            .block_on(self.client.inner.layouts().create(layout))
    }

    /// See [`crate::Layouts::get`].
    pub fn get(&self, permalink: &str) -> Result<Layout> {
        self.client
            .block_on(self.client.inner.layouts().get(permalink))
    }

    /// See [`crate::Layouts::update`].
    pub fn update(&self, permalink: &str, fields: LayoutFields) -> Result<Layout> {
        self.client
            .block_on(self.client.inner.layouts().update(permalink, fields))
    }

    /// See [`crate::Layouts::delete`].
    pub fn delete(&self, permalink: &str) -> Result<DeleteResult> {
        self.client
            .block_on(self.client.inner.layouts().delete(permalink))
    }

    /// See [`crate::Layouts::upload_logo`].
    pub fn upload_logo(&self, permalink: &str, data_url: &str) -> Result<LayoutLogo> {
        self.client
            .block_on(self.client.inner.layouts().upload_logo(permalink, data_url))
    }
}

/// Blocking counterpart of [`crate::Inbound`].
#[derive(Debug, Clone, Copy)]
pub struct Inbound<'a> {
    client: &'a CamelMailer,
}

impl Inbound<'_> {
    /// See [`crate::Inbound::list`].
    pub fn list(&self, params: ListInboundParams) -> Result<InboundList> {
        self.client
            .block_on(self.client.inner.inbound().list(params))
    }

    /// See [`crate::Inbound::get`].
    pub fn get(&self, id: i64) -> Result<Message> {
        self.client.block_on(self.client.inner.inbound().get(id))
    }

    /// See [`crate::Inbound::retry`].
    pub fn retry(&self, id: i64) -> Result<RequeueResult> {
        self.client.block_on(self.client.inner.inbound().retry(id))
    }

    /// See [`crate::Inbound::bypass`].
    pub fn bypass(&self, id: i64) -> Result<RequeueResult> {
        self.client.block_on(self.client.inner.inbound().bypass(id))
    }
}

/// Blocking counterpart of [`crate::Logs`].
#[derive(Debug, Clone, Copy)]
pub struct Logs<'a> {
    client: &'a CamelMailer,
}

impl Logs<'_> {
    /// See [`crate::Logs::list`].
    pub fn list(&self, params: ListLogsParams) -> Result<LogList> {
        self.client.block_on(self.client.inner.logs().list(params))
    }

    /// See [`crate::Logs::tags`].
    pub fn tags(&self) -> Result<Vec<TagCount>> {
        self.client.block_on(self.client.inner.logs().tags())
    }
}
