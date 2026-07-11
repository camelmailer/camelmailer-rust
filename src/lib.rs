//! Rust SDK for [CamelMailer](https://camelmailer.com) — the open-source
//! transactional email platform.
//!
//! Async-first (reqwest + rustls), fully typed, with an optional
//! [`blocking`] client behind the `blocking` feature.
//!
//! # Quickstart
//!
//! ```no_run
//! use camelmailer_rs::{CamelMailer, SendEmailRequest};
//!
//! # async fn run() -> Result<(), camelmailer_rs::Error> {
//! let client = CamelMailer::new("cm_xxxxxxxx");
//!
//! let result = client
//!     .emails()
//!     .send(
//!         SendEmailRequest::builder()
//!             .from("billing@acme.com")
//!             .to("ada@example.com")
//!             .subject("Your receipt")
//!             .html_body("<h1>Thanks for your purchase!</h1>")
//!             .tag("receipt")
//!             .build(),
//!     )
//!     .await?;
//!
//! println!("queued as message {:?}", result.message_id);
//! # Ok(())
//! # }
//! ```
//!
//! # Self-hosted instances
//!
//! The client talks to the CamelMailer cloud
//! (`https://app.camelmailer.com`) by default. Point it at your own
//! instance with the builder:
//!
//! ```no_run
//! use camelmailer_rs::CamelMailer;
//!
//! let client = CamelMailer::builder("cm_xxxxxxxx")
//!     .base_url("https://mail.example.com")
//!     .build()
//!     .expect("valid configuration");
//! ```
//!
//! # Error handling
//!
//! Every API error carries the stable error code from the response
//! envelope:
//!
//! ```no_run
//! use camelmailer_rs::{CamelMailer, Error};
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! # let client = CamelMailer::new("cm_xxxxxxxx");
//! match client.emails().get(42).await {
//!     Ok(detail) => println!("status: {:?}", detail.message.status),
//!     Err(Error::Api { code, message, status }) => {
//!         eprintln!("API said no: {code} ({status}): {message}");
//!     }
//!     Err(other) => return Err(other.into()),
//! }
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod bounces;
mod dmarc;
mod emails;
mod error;
mod http;
mod servers;
mod stats;
mod streams;
mod templates;
mod types;

#[cfg(feature = "blocking")]
pub mod blocking;

pub use bounces::{BounceList, Bounces, ListBouncesParams};
pub use dmarc::{
    Dmarc, DmarcParams, DmarcRecord, DmarcReport, DmarcReportDetail, DmarcReportList, DmarcSource,
    DmarcSummary,
};
pub use emails::{
    ActivityEvent, BatchEntry, BatchError, Delivery, Emails, ListMessagesParams, Message,
    MessageDetail, MessageList, RawMessage, SendEmailRequest, SendEmailRequestBuilder,
    SendRecipient, SendResult, SendTemplateRequest, SendTemplateRequestBuilder,
};
pub use error::{Error, Result};
pub use servers::{Ping, Server, Servers};
pub use stats::{DeliveryStats, DomainQueue, Stats, StatsParams, StatsService};
pub use streams::{CreateStreamRequest, Stream, Streams, UpdateStreamRequest};
pub use templates::{RenderedTemplate, Template, TemplateFields, Templates};
pub use types::{Address, Attachment, Headers, Pagination};

use crate::http::Http;

/// The default base URL: the CamelMailer cloud.
pub const DEFAULT_BASE_URL: &str = "https://app.camelmailer.com";

/// The async CamelMailer client (messaging API, `X-Server-API-Key`).
///
/// Cheap to clone; clones share the same connection pool.
#[derive(Debug, Clone)]
pub struct CamelMailer {
    http: Http,
}

impl CamelMailer {
    /// A client for the CamelMailer cloud with default settings.
    ///
    /// `api_key` is a server API credential (`cm_…`). Use
    /// [`CamelMailer::builder`] to target a self-hosted instance.
    pub fn new(api_key: impl Into<String>) -> Self {
        Self::builder(api_key)
            .build()
            .expect("default configuration is valid")
    }

    /// Start configuring a client.
    pub fn builder(api_key: impl Into<String>) -> CamelMailerBuilder {
        CamelMailerBuilder {
            api_key: api_key.into(),
            base_url: DEFAULT_BASE_URL.to_string(),
            timeout: None,
            client: None,
        }
    }

    /// The base URL this client talks to (no trailing slash).
    pub fn base_url(&self) -> &str {
        self.http.base_url()
    }

    /// Send and inspect messages.
    pub fn emails(&self) -> Emails<'_> {
        Emails { http: &self.http }
    }

    /// Manage and render templates.
    pub fn templates(&self) -> Templates<'_> {
        Templates { http: &self.http }
    }

    /// Manage message streams.
    pub fn streams(&self) -> Streams<'_> {
        Streams { http: &self.http }
    }

    /// Message and queue statistics.
    pub fn stats(&self) -> StatsService<'_> {
        StatsService { http: &self.http }
    }

    /// Bounced messages.
    pub fn bounces(&self) -> Bounces<'_> {
        Bounces { http: &self.http }
    }

    /// DMARC monitoring.
    pub fn dmarc(&self) -> Dmarc<'_> {
        Dmarc { http: &self.http }
    }

    /// The authenticated server (show / ping).
    pub fn server(&self) -> Servers<'_> {
        Servers { http: &self.http }
    }
}

/// Builder for [`CamelMailer`]. Created with [`CamelMailer::builder`].
#[derive(Debug)]
pub struct CamelMailerBuilder {
    api_key: String,
    base_url: String,
    timeout: Option<std::time::Duration>,
    client: Option<reqwest::Client>,
}

impl CamelMailerBuilder {
    /// Base URL of the instance, e.g. `https://mail.example.com`.
    ///
    /// Defaults to [`DEFAULT_BASE_URL`]. A trailing slash is ignored.
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Request timeout (no timeout by default).
    pub fn timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Bring your own [`reqwest::Client`] (proxies, extra roots, …).
    /// Overrides [`CamelMailerBuilder::timeout`].
    pub fn http_client(mut self, client: reqwest::Client) -> Self {
        self.client = Some(client);
        self
    }

    /// Build the client. Fails on an unparsable base URL or when the
    /// TLS backend cannot be initialized.
    pub fn build(self) -> Result<CamelMailer> {
        let base_url = self.base_url.trim_end_matches('/').to_string();
        let parsed = url::Url::parse(&base_url)
            .map_err(|error| Error::Config(format!("invalid base URL {base_url:?}: {error}")))?;
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err(Error::Config(format!(
                "base URL must be http(s), got {base_url:?}"
            )));
        }
        let client = match self.client {
            Some(client) => client,
            None => {
                let mut builder = reqwest::Client::builder()
                    .user_agent(concat!("camelmailer-rs/", env!("CARGO_PKG_VERSION")));
                if let Some(timeout) = self.timeout {
                    builder = builder.timeout(timeout);
                }
                builder
                    .build()
                    .map_err(|error| Error::Config(error.to_string()))?
            }
        };
        Ok(CamelMailer {
            http: Http::new(client, base_url, self.api_key),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_base_url_is_the_cloud() {
        let client = CamelMailer::new("cm_test");
        assert_eq!(client.base_url(), "https://app.camelmailer.com");
    }

    #[test]
    fn builder_trims_trailing_slash() {
        let client = CamelMailer::builder("cm_test")
            .base_url("https://mail.example.com/")
            .build()
            .unwrap();
        assert_eq!(client.base_url(), "https://mail.example.com");
    }

    #[test]
    fn builder_rejects_invalid_base_url() {
        let error = CamelMailer::builder("cm_test")
            .base_url("not a url")
            .build()
            .unwrap_err();
        assert!(matches!(error, Error::Config(_)));
    }

    #[test]
    fn builder_rejects_non_http_scheme() {
        let error = CamelMailer::builder("cm_test")
            .base_url("ftp://mail.example.com")
            .build()
            .unwrap_err();
        assert!(matches!(error, Error::Config(_)));
    }
}
