//! Broadcast campaigns (`/api/v2/server/campaigns`).

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::http::Http;

/// Service for broadcast campaigns. Obtain via
/// [`CamelMailer::campaigns`](crate::CamelMailer::campaigns).
///
/// A campaign is content plus an audience. There are two ways to create
/// one and they behave differently: [`Campaigns::create_draft`] writes
/// it and waits, while [`Campaigns::create_and_send`] expands it to the
/// stream's subscribers before the call returns.
#[derive(Debug, Clone, Copy)]
pub struct Campaigns<'a> {
    pub(crate) http: &'a Http,
}

impl Campaigns<'_> {
    /// Every campaign of the server, newest first
    /// (`GET /api/v2/server/campaigns`).
    pub async fn list(&self) -> Result<Vec<Campaign>> {
        let response: CampaignsResponse = self
            .http
            .get("/api/v2/server/campaigns", None::<&()>)
            .await?;
        Ok(response.campaigns)
    }

    /// The campaigns of one broadcast stream
    /// (`GET /api/v2/server/streams/{permalink}/campaigns`).
    pub async fn list_for_stream(&self, permalink: &str) -> Result<Vec<Campaign>> {
        let response: CampaignsResponse = self
            .http
            .get(
                &format!("/api/v2/server/streams/{permalink}/campaigns"),
                None::<&()>,
            )
            .await?;
        Ok(response.campaigns)
    }

    /// One campaign together with its statistics
    /// (`GET /api/v2/server/campaigns/{id}`).
    pub async fn get(&self, id: i64) -> Result<CampaignDetail> {
        self.http
            .get(&format!("/api/v2/server/campaigns/{id}"), None::<&()>)
            .await
    }

    /// One campaign through its stream
    /// (`GET /api/v2/server/streams/{permalink}/campaigns/{id}`).
    pub async fn get_for_stream(&self, permalink: &str, id: i64) -> Result<CampaignDetail> {
        self.http
            .get(
                &format!("/api/v2/server/streams/{permalink}/campaigns/{id}"),
                None::<&()>,
            )
            .await
    }

    /// Create a campaign without sending it
    /// (`POST /api/v2/server/campaigns`).
    ///
    /// Name the audience with `stream`. Leave `scheduled_at` unset for a
    /// draft, set it for a scheduled send, or set `send_now` to send on
    /// creation.
    pub async fn create_draft(&self, campaign: CreateDraftCampaign) -> Result<Campaign> {
        let response: CampaignResponse = self
            .http
            .post("/api/v2/server/campaigns", Some(&campaign))
            .await?;
        Ok(response.campaign)
    }

    /// Create a campaign on a broadcast stream and send it immediately
    /// (`POST /api/v2/server/streams/{permalink}/campaigns`).
    ///
    /// The send starts before this call returns, so there is no draft to
    /// review and no schedule to set. Use [`Campaigns::create_draft`]
    /// when the campaign should wait.
    pub async fn create_and_send(
        &self,
        permalink: &str,
        campaign: CreateAndSendCampaign,
    ) -> Result<Campaign> {
        let response: CampaignResponse = self
            .http
            .post(
                &format!("/api/v2/server/streams/{permalink}/campaigns"),
                Some(&campaign),
            )
            .await?;
        Ok(response.campaign)
    }

    /// Update a draft or scheduled campaign
    /// (`PATCH /api/v2/server/campaigns/{id}`).
    ///
    /// A campaign that is already sending cannot be edited and the API
    /// answers `ValidationError`.
    pub async fn update(&self, id: i64, fields: UpdateCampaign) -> Result<Campaign> {
        let response: CampaignResponse = self
            .http
            .patch(&format!("/api/v2/server/campaigns/{id}"), Some(&fields))
            .await?;
        Ok(response.campaign)
    }

    /// Send a campaign now, whatever its schedule said
    /// (`POST /api/v2/server/campaigns/{id}/send`).
    pub async fn send(&self, id: i64) -> Result<Campaign> {
        let response: CampaignResponse = self
            .http
            .post(&format!("/api/v2/server/campaigns/{id}/send"), None::<&()>)
            .await?;
        Ok(response.campaign)
    }

    /// Cancel a scheduled or in-flight campaign
    /// (`POST /api/v2/server/campaigns/{id}/cancel`).
    ///
    /// Messages already queued are not recalled.
    pub async fn cancel(&self, id: i64) -> Result<Campaign> {
        let response: CampaignResponse = self
            .http
            .post(
                &format!("/api/v2/server/campaigns/{id}/cancel"),
                None::<&()>,
            )
            .await?;
        Ok(response.campaign)
    }
}

/// Fields for [`Campaigns::create_draft`].
#[derive(Debug, Clone, Default, Serialize)]
pub struct CreateDraftCampaign {
    /// Permalink of the broadcast stream to send to.
    pub stream: String,
    /// Bare sender address; the broadcast path authorizes its domain.
    pub from: String,
    /// Display name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Message subject.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    /// HTML part.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html_body: Option<String>,
    /// Plain-text part.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_body: Option<String>,
    /// RFC 3339 send time; arms the campaign as `scheduled`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled_at: Option<String>,
    /// Send on creation, overriding `scheduled_at`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub send_now: Option<bool>,
}

impl CreateDraftCampaign {
    /// A draft on the given stream, from the given address.
    pub fn new(stream: impl Into<String>, from: impl Into<String>) -> Self {
        Self {
            stream: stream.into(),
            from: from.into(),
            ..Default::default()
        }
    }

    /// Display name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Message subject.
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    /// HTML part.
    pub fn html_body(mut self, html_body: impl Into<String>) -> Self {
        self.html_body = Some(html_body.into());
        self
    }

    /// Plain-text part.
    pub fn text_body(mut self, text_body: impl Into<String>) -> Self {
        self.text_body = Some(text_body.into());
        self
    }

    /// RFC 3339 send time; arms the campaign as `scheduled`.
    pub fn scheduled_at(mut self, scheduled_at: impl Into<String>) -> Self {
        self.scheduled_at = Some(scheduled_at.into());
        self
    }

    /// Send on creation, overriding any schedule.
    pub fn send_now(mut self, send_now: bool) -> Self {
        self.send_now = Some(send_now);
        self
    }
}

/// Fields for [`Campaigns::create_and_send`]. There is no schedule
/// here: the send starts before the call returns.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CreateAndSendCampaign {
    /// Display name.
    pub name: String,
    /// Bare sender address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    /// Message subject.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    /// HTML part.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html_body: Option<String>,
    /// Plain-text part.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_body: Option<String>,
}

impl CreateAndSendCampaign {
    /// An immediate campaign with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Bare sender address.
    pub fn from(mut self, from: impl Into<String>) -> Self {
        self.from = Some(from.into());
        self
    }

    /// Message subject.
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    /// HTML part.
    pub fn html_body(mut self, html_body: impl Into<String>) -> Self {
        self.html_body = Some(html_body.into());
        self
    }

    /// Plain-text part.
    pub fn text_body(mut self, text_body: impl Into<String>) -> Self {
        self.text_body = Some(text_body.into());
        self
    }
}

/// Fields for [`Campaigns::update`]. All optional.
///
/// The schedule is three-valued, which is why it has two methods:
/// leaving it alone keeps the current schedule,
/// [`UpdateCampaign::scheduled_at`] moves a draft to `scheduled`, and
/// [`UpdateCampaign::clear_schedule`] sends an explicit `null`, which
/// drops the campaign back to `draft`. An omitted field and a `null`
/// mean different things to the API.
#[derive(Debug, Clone, Default, Serialize)]
pub struct UpdateCampaign {
    /// New display name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// New sender address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    /// New subject.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    /// New HTML part.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html_body: Option<String>,
    /// New plain-text part.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_body: Option<String>,
    /// `None` omits the field, `Some(None)` sends an explicit `null`
    /// (which clears the schedule), `Some(Some(t))` schedules for `t`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled_at: Option<Option<String>>,
}

impl UpdateCampaign {
    /// An empty update.
    pub fn new() -> Self {
        Self::default()
    }

    /// New display name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// New sender address.
    pub fn from(mut self, from: impl Into<String>) -> Self {
        self.from = Some(from.into());
        self
    }

    /// New subject.
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    /// New HTML part.
    pub fn html_body(mut self, html_body: impl Into<String>) -> Self {
        self.html_body = Some(html_body.into());
        self
    }

    /// New plain-text part.
    pub fn text_body(mut self, text_body: impl Into<String>) -> Self {
        self.text_body = Some(text_body.into());
        self
    }

    /// Schedule the campaign for an RFC 3339 time.
    pub fn scheduled_at(mut self, scheduled_at: impl Into<String>) -> Self {
        self.scheduled_at = Some(Some(scheduled_at.into()));
        self
    }

    /// Clear the schedule, dropping the campaign back to `draft`. This
    /// sends an explicit `null`; omitting the field would leave the
    /// schedule standing.
    pub fn clear_schedule(mut self) -> Self {
        self.scheduled_at = Some(None);
        self
    }
}

/// The audience stream carried on every campaign, so a list needs no
/// second lookup.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct CampaignStream {
    /// Permalink of the stream.
    pub permalink: String,
    /// Display name of the stream.
    pub name: String,
}

/// A broadcast campaign.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Campaign {
    /// Numeric id.
    pub id: i64,
    /// Display name.
    pub name: Option<String>,
    /// Message subject.
    pub subject: Option<String>,
    /// Sender address.
    pub from: Option<String>,
    /// HTML part.
    pub html_body: Option<String>,
    /// Plain-text part.
    pub text_body: Option<String>,
    /// Lifecycle state: `draft`, `scheduled`, `sending`, `sent`,
    /// `failed` or `canceled`. Only `draft` and `scheduled` are
    /// editable.
    pub status: String,
    /// Recipient count snapshotted when the send begins.
    pub total: i64,
    /// Recipients expanded into messages so far.
    pub sent: i64,
    /// Id of the audience stream.
    pub stream_id: i64,
    /// Permalink and name of the audience stream.
    pub stream: CampaignStream,
    /// Send time of a scheduled campaign.
    pub scheduled_at: Option<String>,
    /// When the campaign row was created.
    pub created_at: Option<String>,
    /// When expansion finished (`sent` or `failed`).
    pub completed_at: Option<String>,
}

/// Per-campaign counters, attributed through the messages the campaign
/// produced.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct CampaignStats {
    /// Recipient count of the campaign.
    pub total: i64,
    /// Messages created.
    pub sent: i64,
    /// Messages delivered.
    pub delivered: i64,
    /// Messages that failed.
    pub failed: i64,
    /// Messages opened at least once.
    pub opened: i64,
    /// Messages with at least one click.
    pub clicked: i64,
    /// Resulting unsubscribes.
    pub unsubscribed: i64,
}

/// A campaign together with its statistics.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct CampaignDetail {
    /// The campaign itself.
    pub campaign: Campaign,
    /// Its counters.
    pub stats: CampaignStats,
}

#[derive(Debug, Deserialize)]
struct CampaignsResponse {
    #[serde(default)]
    campaigns: Vec<Campaign>,
}

#[derive(Debug, Deserialize)]
struct CampaignResponse {
    campaign: Campaign,
}
