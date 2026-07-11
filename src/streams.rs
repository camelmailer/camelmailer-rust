//! Manage message streams (`/api/v2/server/streams`).

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::http::Http;

/// Service for message streams. Obtain via
/// [`CamelMailer::streams`](crate::CamelMailer::streams).
#[derive(Debug, Clone, Copy)]
pub struct Streams<'a> {
    pub(crate) http: &'a Http,
}

impl Streams<'_> {
    /// List all streams (`GET /api/v2/server/streams`).
    pub async fn list(&self) -> Result<Vec<Stream>> {
        let response: StreamsResponse =
            self.http.get("/api/v2/server/streams", None::<&()>).await?;
        Ok(response.streams)
    }

    /// Create a stream (`POST /api/v2/server/streams`).
    pub async fn create(&self, stream: CreateStreamRequest) -> Result<Stream> {
        let response: StreamResponse = self
            .http
            .post("/api/v2/server/streams", Some(&stream))
            .await?;
        Ok(response.stream)
    }

    /// Fetch one stream by permalink
    /// (`GET /api/v2/server/streams/{permalink}`).
    pub async fn get(&self, permalink: &str) -> Result<Stream> {
        let response: StreamResponse = self
            .http
            .get(&format!("/api/v2/server/streams/{permalink}"), None::<&()>)
            .await?;
        Ok(response.stream)
    }

    /// Update a stream (`PATCH /api/v2/server/streams/{permalink}`).
    pub async fn update(&self, permalink: &str, fields: UpdateStreamRequest) -> Result<Stream> {
        let response: StreamResponse = self
            .http
            .patch(
                &format!("/api/v2/server/streams/{permalink}"),
                Some(&fields),
            )
            .await?;
        Ok(response.stream)
    }

    /// Archive a stream
    /// (`POST /api/v2/server/streams/{permalink}/archive`).
    pub async fn archive(&self, permalink: &str) -> Result<Stream> {
        let response: StreamResponse = self
            .http
            .post(
                &format!("/api/v2/server/streams/{permalink}/archive"),
                None::<&()>,
            )
            .await?;
        Ok(response.stream)
    }
}

/// Fields for creating a stream.
#[derive(Debug, Clone, Serialize)]
pub struct CreateStreamRequest {
    /// Display name.
    pub name: String,
    /// `transactional` (default) or `broadcast`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_type: Option<String>,
    /// Explicit permalink (derived from the name when omitted).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permalink: Option<String>,
}

impl CreateStreamRequest {
    /// A transactional stream with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            stream_type: None,
            permalink: None,
        }
    }

    /// `transactional` or `broadcast`.
    pub fn stream_type(mut self, stream_type: impl Into<String>) -> Self {
        self.stream_type = Some(stream_type.into());
        self
    }

    /// Explicit permalink.
    pub fn permalink(mut self, permalink: impl Into<String>) -> Self {
        self.permalink = Some(permalink.into());
        self
    }
}

/// Fields for updating a stream. All optional.
#[derive(Debug, Clone, Default, Serialize)]
pub struct UpdateStreamRequest {
    /// New display name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// New stream type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_type: Option<String>,
    /// Archive / unarchive.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archived: Option<bool>,
}

impl UpdateStreamRequest {
    /// An empty update.
    pub fn new() -> Self {
        Self::default()
    }

    /// New display name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// New stream type.
    pub fn stream_type(mut self, stream_type: impl Into<String>) -> Self {
        self.stream_type = Some(stream_type.into());
        self
    }

    /// Archive / unarchive the stream.
    pub fn archived(mut self, archived: bool) -> Self {
        self.archived = Some(archived);
        self
    }
}

/// A message stream.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Stream {
    /// Numeric id.
    pub id: i64,
    /// Stable UUID.
    pub uuid: String,
    /// Display name.
    pub name: String,
    /// Permalink used when sending (`stream` field).
    pub permalink: String,
    /// `transactional`, `broadcast` or `inbound`.
    pub stream_type: String,
    /// Whether the stream is archived.
    pub archived: bool,
}

#[derive(Debug, Deserialize)]
struct StreamsResponse {
    #[serde(default)]
    streams: Vec<Stream>,
}

#[derive(Debug, Deserialize)]
struct StreamResponse {
    stream: Stream,
}
