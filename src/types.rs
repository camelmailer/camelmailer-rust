//! Shared types used across resources.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// An email address, optionally with a display name.
///
/// Serializes exactly as the API expects: a bare string
/// (`"ada@example.com"`) or an object (`{ "email": …, "name": … }`).
///
/// ```
/// use camelmailer_rs::Address;
///
/// let plain: Address = "ada@example.com".into();
/// let named = Address::with_name("ada@example.com", "Ada Lovelace");
/// assert_eq!(serde_json::to_string(&plain).unwrap(), "\"ada@example.com\"");
/// assert_eq!(
///     serde_json::to_string(&named).unwrap(),
///     "{\"email\":\"ada@example.com\",\"name\":\"Ada Lovelace\"}"
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Address {
    /// A bare address, e.g. `ada@example.com`.
    Email(String),
    /// An address with a display name.
    WithName {
        /// The email address.
        email: String,
        /// The display name.
        name: String,
    },
}

impl Address {
    /// An address with a display name.
    pub fn with_name(email: impl Into<String>, name: impl Into<String>) -> Self {
        Address::WithName {
            email: email.into(),
            name: name.into(),
        }
    }

    /// The bare email address.
    pub fn email(&self) -> &str {
        match self {
            Address::Email(email) => email,
            Address::WithName { email, .. } => email,
        }
    }
}

impl From<&str> for Address {
    fn from(email: &str) -> Self {
        Address::Email(email.to_string())
    }
}

impl From<String> for Address {
    fn from(email: String) -> Self {
        Address::Email(email)
    }
}

impl From<(&str, &str)> for Address {
    fn from((email, name): (&str, &str)) -> Self {
        Address::with_name(email, name)
    }
}

/// A file attached to an outgoing message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attachment {
    /// File name shown to the recipient, e.g. `invoice.pdf`.
    pub name: String,
    /// MIME type, e.g. `application/pdf`.
    pub content_type: String,
    /// Base64-encoded file content.
    pub data_base64: String,
}

impl Attachment {
    /// Build an attachment from raw bytes (encodes them as base64).
    pub fn from_bytes(
        name: impl Into<String>,
        content_type: impl Into<String>,
        bytes: impl AsRef<[u8]>,
    ) -> Self {
        use base64::Engine as _;
        Self {
            name: name.into(),
            content_type: content_type.into(),
            data_base64: base64::engine::general_purpose::STANDARD.encode(bytes.as_ref()),
        }
    }

    /// Build an attachment from already base64-encoded content.
    pub fn from_base64(
        name: impl Into<String>,
        content_type: impl Into<String>,
        data_base64: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            content_type: content_type.into(),
            data_base64: data_base64.into(),
        }
    }
}

/// Pagination info returned by every list endpoint.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Pagination {
    /// The current page (1-based).
    pub page: u64,
    /// Entries per page (max 100).
    pub per_page: u64,
    /// Total number of entries.
    pub total: u64,
    /// Total number of pages.
    pub total_pages: u64,
}

/// Extra HTTP-style headers on an outgoing message.
pub type Headers = BTreeMap<String, String>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn address_deserializes_both_shapes() {
        let plain: Address = serde_json::from_str("\"a@b.com\"").unwrap();
        assert_eq!(plain, Address::Email("a@b.com".into()));
        let named: Address = serde_json::from_str(r#"{"email":"a@b.com","name":"Ada"}"#).unwrap();
        assert_eq!(named, Address::with_name("a@b.com", "Ada"));
        assert_eq!(named.email(), "a@b.com");
    }

    #[test]
    fn attachment_from_bytes_encodes_base64() {
        let attachment = Attachment::from_bytes("hi.txt", "text/plain", b"hello");
        assert_eq!(attachment.data_base64, "aGVsbG8=");
    }
}
