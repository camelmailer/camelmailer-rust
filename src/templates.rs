//! Manage and render message templates (`/api/v2/server/templates`).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::Result;
use crate::http::Http;

/// Service for message templates. Obtain via
/// [`CamelMailer::templates`](crate::CamelMailer::templates).
#[derive(Debug, Clone, Copy)]
pub struct Templates<'a> {
    pub(crate) http: &'a Http,
}

impl Templates<'_> {
    /// List all templates (`GET /api/v2/server/templates`).
    pub async fn list(&self) -> Result<Vec<Template>> {
        let response: TemplatesResponse = self
            .http
            .get("/api/v2/server/templates", None::<&()>)
            .await?;
        Ok(response.templates)
    }

    /// Create a template (`POST /api/v2/server/templates`).
    pub async fn create(&self, template: TemplateFields) -> Result<Template> {
        let response: TemplateResponse = self
            .http
            .post("/api/v2/server/templates", Some(&template))
            .await?;
        Ok(response.template)
    }

    /// Fetch one template by permalink
    /// (`GET /api/v2/server/templates/{permalink}`).
    pub async fn get(&self, permalink: &str) -> Result<Template> {
        let response: TemplateResponse = self
            .http
            .get(
                &format!("/api/v2/server/templates/{permalink}"),
                None::<&()>,
            )
            .await?;
        Ok(response.template)
    }

    /// Update a template (`PATCH /api/v2/server/templates/{permalink}`).
    pub async fn update(&self, permalink: &str, fields: TemplateFields) -> Result<Template> {
        let response: TemplateResponse = self
            .http
            .patch(
                &format!("/api/v2/server/templates/{permalink}"),
                Some(&fields),
            )
            .await?;
        Ok(response.template)
    }

    /// Archive a template
    /// (`POST /api/v2/server/templates/{permalink}/archive`).
    pub async fn archive(&self, permalink: &str) -> Result<Template> {
        let response: TemplateResponse = self
            .http
            .post(
                &format!("/api/v2/server/templates/{permalink}/archive"),
                None::<&()>,
            )
            .await?;
        Ok(response.template)
    }

    /// Render a template against a model without sending (preview)
    /// (`POST /api/v2/server/templates/{permalink}/render`).
    pub async fn render(&self, permalink: &str, model: Value) -> Result<RenderedTemplate> {
        let body = RenderBody {
            template_model: model,
        };
        let response: RenderResponse = self
            .http
            .post(
                &format!("/api/v2/server/templates/{permalink}/render"),
                Some(&body),
            )
            .await?;
        Ok(response.rendered)
    }
}

/// Fields for creating or updating a template.
///
/// `subject`, `html_body` and `text_body` may contain Mustache-style
/// `{{ variables }}`.
///
/// ```
/// use camelmailer_rs::TemplateFields;
///
/// let template = TemplateFields::new("Welcome")
///     .subject("Welcome, {{ name }}!")
///     .html_body("<p>Hi {{ name }} 👋</p>");
/// ```
#[derive(Debug, Clone, Default, Serialize)]
pub struct TemplateFields {
    /// Display name (also seeds the permalink on create).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Subject line template.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    /// HTML body template.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html_body: Option<String>,
    /// Plain-text body template.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_body: Option<String>,
}

impl TemplateFields {
    /// Start with a name (required when creating).
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            ..Self::default()
        }
    }

    /// Fields for a partial update (no name change).
    pub fn update() -> Self {
        Self::default()
    }

    /// Set the display name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Subject line template.
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    /// HTML body template.
    pub fn html_body(mut self, html: impl Into<String>) -> Self {
        self.html_body = Some(html.into());
        self
    }

    /// Plain-text body template.
    pub fn text_body(mut self, text: impl Into<String>) -> Self {
        self.text_body = Some(text.into());
        self
    }
}

/// A stored message template.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Template {
    /// Numeric id.
    pub id: i64,
    /// Stable UUID.
    pub uuid: String,
    /// Display name.
    pub name: String,
    /// Permalink used to reference the template when sending.
    pub permalink: String,
    /// Subject line template.
    pub subject: Option<String>,
    /// HTML body template.
    pub html_body: Option<String>,
    /// Plain-text body template.
    pub text_body: Option<String>,
    /// Whether the template is archived.
    pub archived: bool,
}

/// The output of rendering a template against a model.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct RenderedTemplate {
    /// Rendered subject.
    pub subject: Option<String>,
    /// Rendered HTML body.
    pub html_body: Option<String>,
    /// Rendered plain-text body.
    pub text_body: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TemplatesResponse {
    #[serde(default)]
    templates: Vec<Template>,
}

#[derive(Debug, Deserialize)]
struct TemplateResponse {
    template: Template,
}

#[derive(Debug, Serialize)]
struct RenderBody {
    template_model: Value,
}

#[derive(Debug, Deserialize)]
struct RenderResponse {
    rendered: RenderedTemplate,
}
