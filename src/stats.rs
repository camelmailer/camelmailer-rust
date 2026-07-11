//! Message and delivery statistics (`/api/v2/server/stats`).

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::http::Http;

/// Service for statistics. Obtain via
/// [`CamelMailer::stats`](crate::CamelMailer::stats).
#[derive(Debug, Clone, Copy)]
pub struct StatsService<'a> {
    pub(crate) http: &'a Http,
}

impl StatsService<'_> {
    /// Aggregate message + engagement counters
    /// (`GET /api/v2/server/stats`).
    pub async fn get(&self, params: StatsParams) -> Result<Stats> {
        let response: StatsResponse = self.http.get("/api/v2/server/stats", Some(&params)).await?;
        Ok(response.stats)
    }

    /// Pending outbound queue depth, per domain
    /// (`GET /api/v2/server/stats/deliveries`).
    pub async fn deliveries(&self) -> Result<DeliveryStats> {
        self.http
            .get("/api/v2/server/stats/deliveries", None::<&()>)
            .await
    }
}

/// Optional time window for [`StatsService::get`] (RFC 3339 timestamps).
#[derive(Debug, Clone, Default, Serialize)]
pub struct StatsParams {
    /// Window start (RFC 3339).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    /// Window end (RFC 3339).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
}

impl StatsParams {
    /// No window: all-time counters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Window start (RFC 3339, e.g. `2026-07-01T00:00:00Z`).
    pub fn from(mut self, from: impl Into<String>) -> Self {
        self.from = Some(from.into());
        self
    }

    /// Window end (RFC 3339).
    pub fn to(mut self, to: impl Into<String>) -> Self {
        self.to = Some(to.into());
        self
    }
}

/// Message counters.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Stats {
    /// All messages.
    pub total: u64,
    /// Incoming messages.
    pub incoming: u64,
    /// Outgoing messages.
    pub outgoing: u64,
    /// Sent messages.
    pub sent: u64,
    /// Pending messages.
    pub pending: u64,
    /// Held messages.
    pub held: u64,
    /// Bounced messages.
    pub bounced: u64,
    /// Soft delivery failures.
    pub soft_fail: u64,
    /// Hard delivery failures.
    pub hard_fail: u64,
    /// Total opens.
    pub opens: u64,
    /// Total clicks.
    pub clicks: u64,
    /// Unique opens.
    pub unique_opens: u64,
    /// Unique clicks.
    pub unique_clicks: u64,
}

/// Pending outbound queue depth.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct DeliveryStats {
    /// Messages waiting in the queue.
    pub queued: u64,
    /// Queue depth per recipient domain.
    pub domains: Vec<DomainQueue>,
}

/// Queue depth of one recipient domain.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct DomainQueue {
    /// The recipient domain.
    pub domain: String,
    /// Messages queued for it.
    pub queued: u64,
}

#[derive(Debug, Deserialize)]
struct StatsResponse {
    stats: Stats,
}
