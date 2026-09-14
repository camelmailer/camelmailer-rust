//! The server's own request log and tag index
//! (`/api/v2/server/logs`, `/api/v2/server/tags`).

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::http::Http;
use crate::types::Pagination;

/// Service for the request log and tag index. Obtain via
/// [`CamelMailer::logs`](crate::CamelMailer::logs).
///
/// Useful when a send did not arrive and the question is whether the
/// request ever reached the API, and with what answer.
#[derive(Debug, Clone, Copy)]
pub struct Logs<'a> {
    pub(crate) http: &'a Http,
}

impl Logs<'_> {
    /// Logged API requests, newest first (`GET /api/v2/server/logs`).
    pub async fn list(&self, params: ListLogsParams) -> Result<LogList> {
        self.http.get("/api/v2/server/logs", Some(&params)).await
    }

    /// Tags used by the server's recent messages, most used first
    /// (`GET /api/v2/server/tags`).
    pub async fn tags(&self) -> Result<Vec<TagCount>> {
        let response: TagsResponse = self.http.get("/api/v2/server/tags", None::<&()>).await?;
        Ok(response.tags)
    }
}

/// Filters for [`Logs::list`]. All optional.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ListLogsParams {
    /// Page number (1-based).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    /// Page size, capped at 100.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_page: Option<u32>,
    /// Restrict to one HTTP method.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    /// Substring match on the request path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Restrict to one HTTP status code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
}

impl ListLogsParams {
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

    /// Restrict to one HTTP method.
    pub fn method(mut self, method: impl Into<String>) -> Self {
        self.method = Some(method.into());
        self
    }

    /// Substring match on the request path.
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Restrict to one HTTP status code.
    pub fn status(mut self, status: u16) -> Self {
        self.status = Some(status);
        self
    }
}

/// One logged API request.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ApiRequest {
    /// Numeric log id.
    pub id: i64,
    /// HTTP method.
    pub method: String,
    /// Request path.
    pub path: String,
    /// The status the API answered with.
    pub status_code: u16,
    /// How long the request took, in milliseconds.
    pub duration_ms: i64,
    /// The client's User-Agent header.
    pub user_agent: Option<String>,
    /// When the request arrived.
    pub created_at: Option<String>,
}

/// One page of logged requests.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct LogList {
    /// The page of logged requests.
    pub requests: Vec<ApiRequest>,
    /// The page window.
    pub pagination: Pagination,
}

/// One tag with how often the server's recent messages used it.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct TagCount {
    /// The tag itself.
    pub tag: String,
    /// How many messages carry it.
    pub count: i64,
}

#[derive(Debug, Deserialize)]
struct TagsResponse {
    #[serde(default)]
    tags: Vec<TagCount>,
}
