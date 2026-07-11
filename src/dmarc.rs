//! DMARC monitoring (`/api/v2/server/dmarc`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::http::Http;
use crate::types::Pagination;

/// Service for DMARC aggregate reports. Obtain via
/// [`CamelMailer::dmarc`](crate::CamelMailer::dmarc).
#[derive(Debug, Clone, Copy)]
pub struct Dmarc<'a> {
    pub(crate) http: &'a Http,
}

impl Dmarc<'_> {
    /// DMARC compliance summary over the stored aggregate reports
    /// (`GET /api/v2/server/dmarc/summary`).
    pub async fn summary(&self, params: DmarcParams) -> Result<DmarcSummary> {
        let response: SummaryResponse = self
            .http
            .get("/api/v2/server/dmarc/summary", Some(&params))
            .await?;
        Ok(response.summary)
    }

    /// List stored DMARC aggregate reports, newest first
    /// (`GET /api/v2/server/dmarc/reports`).
    pub async fn reports(&self, params: DmarcParams) -> Result<DmarcReportList> {
        self.http
            .get("/api/v2/server/dmarc/reports", Some(&params))
            .await
    }

    /// Fetch one report with its records
    /// (`GET /api/v2/server/dmarc/reports/{id}`).
    pub async fn report(&self, id: i64) -> Result<DmarcReportDetail> {
        self.http
            .get(&format!("/api/v2/server/dmarc/reports/{id}"), None::<&()>)
            .await
    }
}

/// Filters shared by the DMARC endpoints. All optional.
#[derive(Debug, Clone, Default, Serialize)]
pub struct DmarcParams {
    /// Restrict to one domain.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// Window start (RFC 3339); matches overlapping report ranges.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    /// Window end (RFC 3339).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    /// Page number (report list only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u64>,
    /// Entries per page (report list only, max 100).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_page: Option<u64>,
}

impl DmarcParams {
    /// No filters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Restrict to one domain.
    pub fn domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = Some(domain.into());
        self
    }

    /// Window start (RFC 3339).
    pub fn from(mut self, from: impl Into<String>) -> Self {
        self.from = Some(from.into());
        self
    }

    /// Window end (RFC 3339).
    pub fn to(mut self, to: impl Into<String>) -> Self {
        self.to = Some(to.into());
        self
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

/// Aggregated DMARC compliance figures.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct DmarcSummary {
    /// Messages covered by the reports.
    pub total: u64,
    /// Messages with DKIM **and** SPF aligned.
    pub pass: u64,
    /// Messages failing alignment.
    pub fail: u64,
    /// `pass / total` (0.0–1.0).
    pub pass_rate: f64,
    /// Top sending sources by volume.
    pub by_source: Vec<DmarcSource>,
    /// Message counts per disposition (`none`, `quarantine`, `reject`).
    pub by_disposition: BTreeMap<String, u64>,
}

/// Alignment figures of one sending source.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct DmarcSource {
    /// The source IP address.
    pub source_ip: String,
    /// Messages from this source.
    pub count: u64,
    /// Percentage of messages with SPF aligned.
    pub spf_aligned_pct: f64,
    /// Percentage of messages with DKIM aligned.
    pub dkim_aligned_pct: f64,
    /// Message counts per disposition.
    pub disposition_counts: BTreeMap<String, u64>,
}

/// A stored DMARC aggregate report.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct DmarcReport {
    /// Numeric id.
    pub id: i64,
    /// The reported domain.
    pub domain: String,
    /// Reporting organization name.
    pub org_name: Option<String>,
    /// Reporting organization contact.
    pub org_email: Option<String>,
    /// The reporter's report id.
    pub report_id: String,
    /// Report range start (RFC 3339).
    pub date_range_begin: String,
    /// Report range end (RFC 3339).
    pub date_range_end: String,
    /// When the report was ingested (RFC 3339).
    pub received_at: String,
    /// Number of records in the report.
    pub record_count: u64,
}

/// One row of a DMARC aggregate report.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct DmarcRecord {
    /// Numeric id.
    pub id: i64,
    /// The sending source IP.
    pub source_ip: String,
    /// Message count for this row.
    pub count: u64,
    /// `none`, `quarantine` or `reject`.
    pub disposition: String,
    /// Raw DKIM result, e.g. `pass`.
    pub dkim_result: Option<String>,
    /// Raw SPF result, e.g. `softfail`.
    pub spf_result: Option<String>,
    /// Whether DKIM was aligned.
    pub dkim_aligned: bool,
    /// Whether SPF was aligned.
    pub spf_aligned: bool,
    /// The `From:` header domain.
    pub header_from: Option<String>,
    /// The envelope-from domain.
    pub envelope_from: Option<String>,
}

/// A page of DMARC reports.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct DmarcReportList {
    /// The reports on this page.
    pub reports: Vec<DmarcReport>,
    /// Pagination info.
    pub pagination: Pagination,
}

/// One report with its records.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct DmarcReportDetail {
    /// The report.
    pub report: DmarcReport,
    /// Its records.
    pub records: Vec<DmarcRecord>,
}

#[derive(Debug, Deserialize)]
struct SummaryResponse {
    summary: DmarcSummary,
}
