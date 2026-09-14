//! Template layouts (`/api/v2/server/layouts`).

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::http::Http;
use crate::subscribers::DeleteResult;

/// Service for template layouts. Obtain via
/// [`CamelMailer::layouts`](crate::CamelMailer::layouts).
///
/// A layout wraps every template that uses it, so header, footer and
/// styling live in one place instead of in each template.
#[derive(Debug, Clone, Copy)]
pub struct Layouts<'a> {
    pub(crate) http: &'a Http,
}

impl Layouts<'_> {
    /// All layouts of the server (`GET /api/v2/server/layouts`).
    pub async fn list(&self) -> Result<Vec<Layout>> {
        let response: LayoutsResponse =
            self.http.get("/api/v2/server/layouts", None::<&()>).await?;
        Ok(response.layouts)
    }

    /// Create a layout (`POST /api/v2/server/layouts`).
    ///
    /// `html_wrapper` has to embed the body with `{{{ content }}}`;
    /// anything else is refused with `ValidationError`.
    pub async fn create(&self, layout: LayoutFields) -> Result<Layout> {
        let response: LayoutResponse = self
            .http
            .post("/api/v2/server/layouts", Some(&layout))
            .await?;
        Ok(response.layout)
    }

    /// One layout by permalink
    /// (`GET /api/v2/server/layouts/{permalink}`).
    pub async fn get(&self, permalink: &str) -> Result<Layout> {
        let response: LayoutResponse = self
            .http
            .get(&format!("/api/v2/server/layouts/{permalink}"), None::<&()>)
            .await?;
        Ok(response.layout)
    }

    /// Update a layout; only the given fields change
    /// (`PATCH /api/v2/server/layouts/{permalink}`).
    pub async fn update(&self, permalink: &str, fields: LayoutFields) -> Result<Layout> {
        let response: LayoutResponse = self
            .http
            .patch(
                &format!("/api/v2/server/layouts/{permalink}"),
                Some(&fields),
            )
            .await?;
        Ok(response.layout)
    }

    /// Delete a layout
    /// (`DELETE /api/v2/server/layouts/{permalink}`).
    ///
    /// Templates that referenced it fall back to no wrapper.
    pub async fn delete(&self, permalink: &str) -> Result<DeleteResult> {
        self.http
            .delete(&format!("/api/v2/server/layouts/{permalink}"))
            .await
    }

    /// Upload the layout's logo as a data URL
    /// (`POST /api/v2/server/layouts/{permalink}/logo`).
    ///
    /// Takes `data:image/png;base64,…` and returns the absolute URL to
    /// reference from the wrapper.
    pub async fn upload_logo(&self, permalink: &str, data_url: &str) -> Result<LayoutLogo> {
        let body = LogoRequest { data_url };
        self.http
            .post(
                &format!("/api/v2/server/layouts/{permalink}/logo"),
                Some(&body),
            )
            .await
    }
}

/// Fields for creating or updating a layout. All optional except when
/// creating, where the API requires a name.
#[derive(Debug, Clone, Default, Serialize)]
pub struct LayoutFields {
    /// Display name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Explicit permalink (derived from the name when omitted).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permalink: Option<String>,
    /// Wrapper for the HTML body; embeds it with `{{{ content }}}`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html_wrapper: Option<String>,
    /// Wrapper for the plain-text body.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_wrapper: Option<String>,
}

impl LayoutFields {
    /// An empty set of fields.
    pub fn new() -> Self {
        Self::default()
    }

    /// Display name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Explicit permalink.
    pub fn permalink(mut self, permalink: impl Into<String>) -> Self {
        self.permalink = Some(permalink.into());
        self
    }

    /// Wrapper for the HTML body.
    pub fn html_wrapper(mut self, html_wrapper: impl Into<String>) -> Self {
        self.html_wrapper = Some(html_wrapper.into());
        self
    }

    /// Wrapper for the plain-text body.
    pub fn text_wrapper(mut self, text_wrapper: impl Into<String>) -> Self {
        self.text_wrapper = Some(text_wrapper.into());
        self
    }
}

#[derive(Debug, Serialize)]
struct LogoRequest<'a> {
    data_url: &'a str,
}

/// Where an uploaded logo now lives.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct LayoutLogo {
    /// Absolute URL to reference from the wrapper. Served without
    /// authentication, because mail clients fetch it without a session.
    pub url: String,
}

/// A template layout.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Layout {
    /// Numeric id.
    pub id: i64,
    /// Stable UUID.
    pub uuid: String,
    /// Display name.
    pub name: String,
    /// Permalink used in API calls and by templates.
    pub permalink: String,
    /// Wrapper for the HTML body. Required when creating a layout, so
    /// it is always present.
    pub html_wrapper: String,
    /// Wrapper for the plain-text body. `None` when unset, which is what
    /// a layout created with only an HTML wrapper comes back as.
    pub text_wrapper: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LayoutsResponse {
    #[serde(default)]
    layouts: Vec<Layout>,
}

#[derive(Debug, Deserialize)]
struct LayoutResponse {
    layout: Layout,
}
