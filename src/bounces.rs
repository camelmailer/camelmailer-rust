//! Bounced messages (`/api/v2/server/bounces`).

use serde::{Deserialize, Serialize};

use crate::emails::Message;
use crate::error::Result;
use crate::http::Http;
use crate::types::Pagination;

/// Service for bounces. Obtain via
/// [`CamelMailer::bounces`](crate::CamelMailer::bounces).
#[derive(Debug, Clone, Copy)]
pub struct Bounces<'a> {
    pub(crate) http: &'a Http,
}

impl Bounces<'_> {
    /// List bounced messages, paginated
    /// (`GET /api/v2/server/bounces`).
    pub async fn list(&self, params: ListBouncesParams) -> Result<BounceList> {
        self.http.get("/api/v2/server/bounces", Some(&params)).await
    }

    /// Fetch one bounced message
    /// (`GET /api/v2/server/bounces/{id}`).
    pub async fn get(&self, id: i64) -> Result<Message> {
        let response: BounceResponse = self
            .http
            .get(&format!("/api/v2/server/bounces/{id}"), None::<&()>)
            .await?;
        Ok(response.bounce)
    }
}

/// Pagination for [`Bounces::list`].
#[derive(Debug, Clone, Default, Serialize)]
pub struct ListBouncesParams {
    /// Page number (1-based).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u64>,
    /// Entries per page (max 100).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_page: Option<u64>,
}

impl ListBouncesParams {
    /// First page, server defaults.
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
}

/// A page of bounced messages.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct BounceList {
    /// The bounces on this page.
    pub bounces: Vec<Message>,
    /// Pagination info.
    pub pagination: Pagination,
}

#[derive(Debug, Deserialize)]
struct BounceResponse {
    bounce: Message,
}
