//! Error types returned by the SDK.

/// Convenient alias for `Result<T, camelmailer_rs::Error>`.
pub type Result<T> = std::result::Result<T, Error>;

/// Every way a CamelMailer API call can fail.
///
/// The interesting variant is [`Error::Api`]: the server replied with the
/// standard error envelope (`{ "status": "error", "error": { "code", "message" } }`).
/// Its `code` is stable and safe to branch on — see
/// [the API docs](https://camelmailer.com/docs) for the full list
/// (`Unauthorized`, `Forbidden`, `NotFound`, `ValidationError`,
/// `ParameterMissing`, …).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The API returned an error envelope.
    #[error("CamelMailer API error {code}: {message} (HTTP {status})")]
    Api {
        /// Stable machine-readable error code, e.g. `ValidationError`.
        code: String,
        /// Human-readable description of what went wrong.
        message: String,
        /// The HTTP status code of the response.
        status: u16,
    },

    /// The request never produced a usable HTTP response
    /// (DNS failure, connection refused, timeout, TLS error, …).
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),

    /// A response could not be (de)serialized into the expected shape.
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    /// The server answered with something that is not a CamelMailer
    /// envelope (e.g. an HTML error page from a proxy).
    #[error("unexpected response (HTTP {status}): {body}")]
    UnexpectedResponse {
        /// The HTTP status code of the response.
        status: u16,
        /// The raw (possibly truncated) response body.
        body: String,
    },

    /// The client was configured with invalid values (e.g. an
    /// unparsable base URL).
    #[error("invalid configuration: {0}")]
    Config(String),

    /// A payload field could not be decoded (e.g. invalid base64 in a
    /// raw message body).
    #[error("decode error: {0}")]
    Decode(String),
}

impl Error {
    /// The stable API error code, when this is an [`Error::Api`].
    pub fn code(&self) -> Option<&str> {
        match self {
            Error::Api { code, .. } => Some(code),
            _ => None,
        }
    }

    /// The HTTP status, when this error carries one.
    pub fn status(&self) -> Option<u16> {
        match self {
            Error::Api { status, .. } | Error::UnexpectedResponse { status, .. } => Some(*status),
            _ => None,
        }
    }

    /// `true` when the API rejected the credentials (`Unauthorized`).
    pub fn is_unauthorized(&self) -> bool {
        self.code() == Some("Unauthorized")
    }

    /// `true` when the resource does not exist (`NotFound`).
    pub fn is_not_found(&self) -> bool {
        self.code() == Some("NotFound")
    }

    /// `true` when the request was understood but invalid
    /// (`ValidationError` or `ParameterMissing`).
    pub fn is_validation_error(&self) -> bool {
        matches!(self.code(), Some("ValidationError" | "ParameterMissing"))
    }
}
