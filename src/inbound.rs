//! Inbound and held messages (`/api/v2/server/inbound`).

use serde::{Deserialize, Serialize};

use crate::emails::Message;
use crate::error::Result;
use crate::http::Http;
use crate::types::Pagination;

/// Service for inbound and held messages. Obtain via
/// [`CamelMailer::inbound`](crate::CamelMailer::inbound).
///
/// It covers mail arriving through an inbound route as well as outbound
/// mail the spam filter put on hold, which is why a message here can be
/// either retried or released past the hold.
#[derive(Debug, Clone, Copy)]
pub struct Inbound<'a> {
    pub(crate) http: &'a Http,
}

impl Inbound<'_> {
    /// Inbound and held messages, newest first
    /// (`GET /api/v2/server/inbound`).
    pub async fn list(&self, params: ListInboundParams) -> Result<InboundList> {
        self.http.get("/api/v2/server/inbound", Some(&params)).await
    }

    /// One inbound message (`GET /api/v2/server/inbound/{id}`).
    pub async fn get(&self, id: i64) -> Result<Message> {
        let response: MessageResponse = self
            .http
            .get(&format!("/api/v2/server/inbound/{id}"), None::<&()>)
            .await?;
        Ok(response.message)
    }

    /// Put a message back on the delivery queue
    /// (`POST /api/v2/server/inbound/{id}/retry`), for instance after
    /// fixing the route it should have matched.
    pub async fn retry(&self, id: i64) -> Result<RequeueResult> {
        self.http
            .post(&format!("/api/v2/server/inbound/{id}/retry"), None::<&()>)
            .await
    }

    /// Release a held message past the hold and deliver it
    /// (`POST /api/v2/server/inbound/{id}/bypass`).
    pub async fn bypass(&self, id: i64) -> Result<RequeueResult> {
        self.http
            .post(&format!("/api/v2/server/inbound/{id}/bypass"), None::<&()>)
            .await
    }
}

/// Filters for [`Inbound::list`]. All optional.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ListInboundParams {
    /// Page number (1-based).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    /// Page size, capped at 100.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_page: Option<u32>,
    /// Restrict to one delivery status, e.g. `held`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Restrict to one message stream (by permalink).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<String>,
    /// Substring match on subject and addresses.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
}

impl ListInboundParams {
    /// No filters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Page number.
    pub fn page(mut self, page: u32) -> Self {
        self.page = Some(page);
        self
    }

    /// Page size.
    pub fn per_page(mut self, per_page: u32) -> Self {
        self.per_page = Some(per_page);
        self
    }

    /// Restrict to one status.
    pub fn status(mut self, status: impl Into<String>) -> Self {
        self.status = Some(status.into());
        self
    }

    /// Restrict to one stream.
    pub fn stream(mut self, stream: impl Into<String>) -> Self {
        self.stream = Some(stream.into());
        self
    }

    /// Substring match on subject and addresses.
    pub fn query(mut self, query: impl Into<String>) -> Self {
        self.query = Some(query.into());
        self
    }
}

/// One page of inbound and held messages.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct InboundList {
    /// The page of messages. The API names this key `inbound`.
    pub inbound: Vec<Message>,
    /// The page window.
    pub pagination: Pagination,
}

/// What a retry or bypass did.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct RequeueResult {
    /// Whether the message went back on the delivery queue.
    pub queued: bool,
}

#[derive(Debug, Deserialize)]
struct MessageResponse {
    message: Message,
}
