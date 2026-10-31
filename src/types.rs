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
    /// Makes the part inline: the HTML body shows it as `cid:<content_id>`,
    /// for a logo for example. Letters, digits and `.-_@+$`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_id: Option<String>,
}

impl Attachment {
    /// An inline part the HTML body shows as `cid:<content_id>`, for a logo
    /// or another image (encodes the bytes as base64). Mail clients show it
    /// in place and do not list it as an attachment.
    pub fn inline(
        name: impl Into<String>,
        content_type: impl Into<String>,
        content_id: impl Into<String>,
        bytes: impl AsRef<[u8]>,
    ) -> Self {
        Self {
            content_id: Some(content_id.into()),
            ..Self::from_bytes(name, content_type, bytes)
        }
    }

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
            content_id: None,
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
            content_id: None,
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

#[cfg(test)]
mod attachment_tests {
    use super::Attachment;

    #[test]
    fn inline_parts_carry_a_content_id_and_plain_attachments_none() {
        let inline = serde_json::to_value(Attachment::inline(
            "logo.png",
            "image/png",
            "logo@acme.example",
            [1, 2, 3],
        ))
        .unwrap();
        assert_eq!(inline["content_id"], "logo@acme.example");
        assert_eq!(inline["data_base64"], "AQID");

        let plain =
            serde_json::to_value(Attachment::from_bytes("a.pdf", "application/pdf", [1])).unwrap();
        assert!(plain.get("content_id").is_none());
    }
}
