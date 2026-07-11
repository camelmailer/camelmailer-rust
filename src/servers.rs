//! The authenticated server (`/api/v2/server/`).

use serde::Deserialize;
use serde_json::Value;

use crate::error::Result;
use crate::http::Http;

/// Service for the authenticated server itself. Obtain via
/// [`CamelMailer::server`](crate::CamelMailer::server).
#[derive(Debug, Clone, Copy)]
pub struct Servers<'a> {
    pub(crate) http: &'a Http,
}

impl Servers<'_> {
    /// Show the server the API key belongs to
    /// (`GET /api/v2/server/`).
    pub async fn get(&self) -> Result<Server> {
        let response: ServerResponse = self.http.get("/api/v2/server/", None::<&()>).await?;
        Ok(response.server)
    }

    /// Validate the API key (`GET /api/v2/server/ping`).
    pub async fn ping(&self) -> Result<Ping> {
        self.http.get("/api/v2/server/ping", None::<&()>).await
    }
}

/// The mail server an API key belongs to.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Server {
    /// Numeric id.
    pub id: i64,
    /// Stable UUID.
    pub uuid: String,
    /// Display name.
    pub name: String,
    /// Permalink.
    pub permalink: String,
    /// `Live` or `Development`.
    pub mode: String,
    /// Whether sending is suspended.
    pub suspended: bool,
    /// Why sending was suspended.
    pub suspension_reason: Option<String>,
    /// Whether privacy mode is on.
    pub privacy_mode: bool,
    /// Whether open tracking is on.
    pub track_opens: bool,
    /// Whether click tracking is on.
    pub track_clicks: bool,
    /// Inbound spam threshold.
    pub spam_threshold: Option<f64>,
    /// Outbound spam threshold.
    pub outbound_spam_threshold: Option<f64>,
    /// Legacy bounce webhook URL.
    pub bounce_hook_url: Option<String>,
    /// Legacy delivery webhook URL.
    pub delivery_hook_url: Option<String>,
    /// Domain accepting inbound mail.
    pub inbound_domain: Option<String>,
    /// Dashboard color.
    pub color: Option<String>,
    /// Assigned IP pool.
    pub ip_pool_id: Option<i64>,
    /// Default message stream.
    pub default_stream_id: Option<i64>,
    /// Any additional fields newer servers may return.
    #[serde(flatten)]
    pub extra: Value,
}

/// Response of the key check.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Ping {
    /// Always `true`.
    pub pong: bool,
    /// Id of the authenticated server.
    pub server_id: i64,
    /// Permalink of the authenticated server.
    pub server: String,
}

#[derive(Debug, Deserialize)]
struct ServerResponse {
    server: Server,
}
