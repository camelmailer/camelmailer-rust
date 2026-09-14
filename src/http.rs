//! Internal HTTP transport: envelope handling, auth header, URL building.

use reqwest::{Method, StatusCode};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{Error, Result};

/// Header carrying the messaging (server) API key.
pub(crate) const SERVER_API_KEY_HEADER: &str = "X-Server-API-Key";

/// Header that makes a send replayable. It is a header rather than a
/// body field because the body is what the server hashes to recognise
/// the same request.
pub(crate) const IDEMPOTENCY_KEY_HEADER: &str = "Idempotency-Key";

/// How much of a non-envelope body to keep in error messages.
const BODY_SNIPPET_LEN: usize = 512;

#[derive(Debug, Deserialize)]
struct Envelope {
    status: String,
    #[serde(default)]
    data: Option<Value>,
    #[serde(default)]
    error: Option<ErrorBody>,
}

#[derive(Debug, Deserialize)]
struct ErrorBody {
    #[serde(default)]
    code: String,
    #[serde(default)]
    message: String,
}

/// The shared transport behind [`crate::CamelMailer`] and its services.
#[derive(Debug, Clone)]
pub(crate) struct Http {
    client: reqwest::Client,
    /// Base URL without a trailing slash, e.g. `https://app.camelmailer.com`.
    base_url: String,
    api_key: String,
}

impl Http {
    pub(crate) fn new(client: reqwest::Client, base_url: String, api_key: String) -> Self {
        Self {
            client,
            base_url,
            api_key,
        }
    }

    pub(crate) fn base_url(&self) -> &str {
        &self.base_url
    }

    pub(crate) async fn get<T: DeserializeOwned>(
        &self,
        path: &str,
        query: Option<&impl Serialize>,
    ) -> Result<T> {
        self.request(Method::GET, path, query, None::<&()>).await
    }

    pub(crate) async fn post<T: DeserializeOwned>(
        &self,
        path: &str,
        body: Option<&impl Serialize>,
    ) -> Result<T> {
        self.request(Method::POST, path, None::<&()>, body).await
    }

    /// POST with an optional `Idempotency-Key`.
    pub(crate) async fn post_idempotent<T: DeserializeOwned>(
        &self,
        path: &str,
        body: Option<&impl Serialize>,
        idempotency_key: Option<&str>,
    ) -> Result<T> {
        self.send(Method::POST, path, None::<&()>, body, idempotency_key)
            .await
    }

    pub(crate) async fn delete<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.request(Method::DELETE, path, None::<&()>, None::<&()>)
            .await
    }

    pub(crate) async fn patch<T: DeserializeOwned>(
        &self,
        path: &str,
        body: Option<&impl Serialize>,
    ) -> Result<T> {
        self.request(Method::PATCH, path, None::<&()>, body).await
    }

    async fn request<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        query: Option<&impl Serialize>,
        body: Option<&impl Serialize>,
    ) -> Result<T> {
        self.send(method, path, query, body, None).await
    }

    async fn send<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        query: Option<&impl Serialize>,
        body: Option<&impl Serialize>,
        idempotency_key: Option<&str>,
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let mut request = self
            .client
            .request(method, url)
            .header(SERVER_API_KEY_HEADER, &self.api_key);
        if let Some(key) = idempotency_key {
            request = request.header(IDEMPOTENCY_KEY_HEADER, key);
        }
        if let Some(query) = query {
            request = request.query(query);
        }
        if let Some(body) = body {
            request = request.json(body);
        }
        let response = request.send().await?;
        let status = response.status();
        let text = response.text().await?;
        Self::parse_envelope(status, &text)
    }

    fn parse_envelope<T: DeserializeOwned>(status: StatusCode, text: &str) -> Result<T> {
        let envelope: Envelope = match serde_json::from_str(text) {
            Ok(envelope) => envelope,
            Err(_) => {
                return Err(Error::UnexpectedResponse {
                    status: status.as_u16(),
                    body: snippet(text),
                })
            }
        };
        if envelope.status == "error" || !status.is_success() {
            let error = envelope.error.unwrap_or_else(|| ErrorBody {
                code: format!("Http{}", status.as_u16()),
                message: snippet(text),
            });
            return Err(Error::Api {
                code: error.code,
                message: error.message,
                status: status.as_u16(),
            });
        }
        let data = envelope.data.unwrap_or(Value::Null);
        Ok(serde_json::from_value(data)?)
    }
}

fn snippet(text: &str) -> String {
    let mut end = BODY_SNIPPET_LEN.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_envelope_yields_data() {
        let value: Value = Http::parse_envelope(
            StatusCode::OK,
            r#"{"status":"success","time":0.1,"data":{"pong":true}}"#,
        )
        .expect("parses");
        assert_eq!(value["pong"], Value::Bool(true));
    }

    #[test]
    fn error_envelope_yields_api_error() {
        let result: Result<Value> = Http::parse_envelope(
            StatusCode::UNPROCESSABLE_ENTITY,
            r#"{"status":"error","time":0.1,"error":{"code":"ValidationError","message":"nope"}}"#,
        );
        match result.unwrap_err() {
            Error::Api {
                code,
                message,
                status,
            } => {
                assert_eq!(code, "ValidationError");
                assert_eq!(message, "nope");
                assert_eq!(status, 422);
            }
            other => panic!("expected Error::Api, got {other:?}"),
        }
    }

    #[test]
    fn non_envelope_body_is_unexpected_response() {
        let result: Result<Value> =
            Http::parse_envelope(StatusCode::BAD_GATEWAY, "<html>Bad gateway</html>");
        match result.unwrap_err() {
            Error::UnexpectedResponse { status, body } => {
                assert_eq!(status, 502);
                assert!(body.contains("Bad gateway"));
            }
            other => panic!("expected UnexpectedResponse, got {other:?}"),
        }
    }

    #[test]
    fn http_error_with_success_body_still_fails() {
        // A proxy could rewrite the status; never treat a non-2xx as success.
        let result: Result<Value> = Http::parse_envelope(
            StatusCode::BAD_GATEWAY,
            r#"{"status":"success","time":0.1}"#,
        );
        assert!(matches!(
            result.unwrap_err(),
            Error::Api { status: 502, .. }
        ));
    }
}
