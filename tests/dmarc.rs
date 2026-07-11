//! Wiremock tests for the DMARC service.

mod common;

use camelmailer_rs::DmarcParams;
use common::{client, error, success};
use serde_json::json;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn report_json() -> serde_json::Value {
    json!({
        "id": 12,
        "domain": "acme.com",
        "org_name": "google.com",
        "org_email": "noreply-dmarc-support@google.com",
        "report_id": "1234567890",
        "date_range_begin": "2026-07-09T00:00:00+00:00",
        "date_range_end": "2026-07-10T00:00:00+00:00",
        "received_at": "2026-07-10T04:00:00+00:00",
        "record_count": 2,
    })
}

#[tokio::test]
async fn summary_sends_filters_and_parses() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/server/dmarc/summary"))
        .and(query_param("domain", "acme.com"))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
            "summary": {
                "total": 200,
                "pass": 180,
                "fail": 20,
                "pass_rate": 0.9,
                "by_source": [{
                    "source_ip": "203.0.113.10",
                    "count": 150,
                    "spf_aligned_pct": 98.0,
                    "dkim_aligned_pct": 99.3,
                    "disposition_counts": { "none": 150 },
                }],
                "by_disposition": { "none": 180, "quarantine": 20 },
            },
        }))))
        .expect(1)
        .mount(&server)
        .await;

    let summary = client(&server)
        .dmarc()
        .summary(DmarcParams::new().domain("acme.com"))
        .await
        .unwrap();

    assert_eq!(summary.total, 200);
    assert!((summary.pass_rate - 0.9).abs() < f64::EPSILON);
    assert_eq!(summary.by_source[0].source_ip, "203.0.113.10");
    assert_eq!(summary.by_disposition.get("quarantine"), Some(&20));
}

#[tokio::test]
async fn reports_list_paginates() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/server/dmarc/reports"))
        .and(query_param("page", "1"))
        .and(query_param("per_page", "25"))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
            "reports": [report_json()],
            "pagination": { "page": 1, "per_page": 25, "total": 1, "total_pages": 1 },
        }))))
        .expect(1)
        .mount(&server)
        .await;

    let list = client(&server)
        .dmarc()
        .reports(DmarcParams::new().page(1).per_page(25))
        .await
        .unwrap();

    assert_eq!(list.reports.len(), 1);
    assert_eq!(list.reports[0].domain, "acme.com");
    assert_eq!(list.reports[0].record_count, 2);
}

#[tokio::test]
async fn report_detail_includes_records() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/server/dmarc/reports/12"))
        .respond_with(ResponseTemplate::new(200).set_body_json(success(json!({
            "report": report_json(),
            "records": [{
                "id": 1,
                "source_ip": "203.0.113.10",
                "count": 150,
                "disposition": "none",
                "dkim_result": "pass",
                "spf_result": "softfail",
                "dkim_aligned": true,
                "spf_aligned": false,
                "header_from": "acme.com",
                "envelope_from": "bounce.acme.com",
            }],
        }))))
        .mount(&server)
        .await;

    let detail = client(&server).dmarc().report(12).await.unwrap();
    assert_eq!(detail.report.id, 12);
    assert_eq!(detail.records.len(), 1);
    assert!(detail.records[0].dkim_aligned);
    assert!(!detail.records[0].spf_aligned);
}

#[tokio::test]
async fn missing_report_is_not_found() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/server/dmarc/reports/999"))
        .respond_with(ResponseTemplate::new(404).set_body_json(error("NotFound", "not found")))
        .mount(&server)
        .await;

    let err = client(&server).dmarc().report(999).await.unwrap_err();
    assert!(err.is_not_found());
}
