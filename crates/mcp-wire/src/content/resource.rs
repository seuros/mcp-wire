//! Resource content type for MCP resources/read responses
//!
//! `ResourceContent` automatically includes uri and mimeType fields as required
//! by the MCP spec for resources/read responses.

use std::borrow::Cow;

use serde::de::Error as _;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};

/// Default MIME type of [`ResourceContent::text`].
const TEXT_PLAIN: &str = "text/plain";

/// Resource content for MCP resources/read responses
///
/// Automatically includes uri and mimeType per MCP spec.
/// Use this type for `Resource::read()` return values.
///
/// # Examples
///
/// ```rust
/// use mcp_wire::content::resource::ResourceContent;
///
/// // Simple text resource
/// let text = ResourceContent::text("file:///path", "content here");
///
/// // With specific MIME type
/// let json = ResourceContent::text("file:///path", "{}").with_mime_type("application/json");
///
/// // Binary resource
/// let base64_data = String::from("iVBORw==");
/// let image = ResourceContent::blob("file:///path", base64_data, "image/png");
/// assert_eq!(text.mime_type.as_deref(), Some("text/plain"));
/// assert_eq!(json.mime_type.as_deref(), Some("application/json"));
/// assert_eq!(image.mime_type.as_deref(), Some("image/png"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceContent {
    /// Resource URI (required)
    pub uri: String,

    /// MIME type; constant types are borrowed, not allocated per message.
    pub mime_type: Option<Cow<'static, str>>,

    /// The `text` or `blob` payload; the schema allows exactly one.
    pub payload: ResourcePayload,

    /// Protocol metadata associated with the resource contents.
    pub meta: Option<Map<String, Value>>,
}

/// [`ResourceContent`] payload: `TextResourceContents` or `BlobResourceContents`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourcePayload {
    /// Text, sent as `text`.
    Text(String),
    /// Base64-encoded bytes, sent as `blob`.
    Blob(String),
}

impl ResourcePayload {
    /// The wire key: `text` or `blob`.
    const fn key(&self) -> &'static str {
        match self {
            Self::Text(_) => "text",
            Self::Blob(_) => "blob",
        }
    }
}

impl ResourceContent {
    /// Contents with no MIME type or metadata.
    pub fn new(uri: impl Into<String>, payload: ResourcePayload) -> Self {
        Self {
            uri: uri.into(),
            mime_type: None,
            payload,
            meta: None,
        }
    }

    /// Convert these resource contents to their resources/read wire value.
    pub fn to_value(&self) -> Value {
        self.clone().into_value()
    }

    /// Convert to JSON, moving the resource payload and metadata.
    pub fn into_value(self) -> Value {
        let mut object = Map::new();
        object.insert("uri".into(), Value::String(self.uri));
        super::types::insert_string(&mut object, "mimeType", self.mime_type);
        let key = self.payload.key();
        let (ResourcePayload::Text(payload) | ResourcePayload::Blob(payload)) = self.payload;
        object.insert(key.into(), Value::String(payload));
        super::types::insert_object(&mut object, "_meta", self.meta);
        Value::Object(object)
    }

    /// Create a text resource with default text/plain MIME type
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_wire::content::resource::ResourceContent;
    ///
    /// let content = ResourceContent::text("file:///doc.txt", "Hello, world!");
    /// assert_eq!(content.uri, "file:///doc.txt");
    /// ```
    pub fn text(uri: impl Into<String>, text: impl Into<String>) -> Self {
        Self::new(uri, ResourcePayload::Text(text.into())).with_mime_type(TEXT_PLAIN)
    }

    /// Create an MCP Apps view (`text/html;profile=mcp-app`)
    ///
    /// ```rust
    /// use mcp_wire::content::resource::ResourceContent;
    /// use mcp_wire::protocol::apps::UiResourceMeta;
    ///
    /// let ui = UiResourceMeta { prefers_border: Some(true), ..Default::default() };
    /// let content = ResourceContent::mcp_app("ui://weather/dashboard", "<!doctype html>")
    ///     .with_meta(ui.into_meta()?);
    /// assert_eq!(content.mime_type.as_deref(), Some("text/html;profile=mcp-app"));
    /// # Ok::<(), mcp_wire::protocol::apps::AppsError>(())
    /// ```
    pub fn mcp_app(uri: impl Into<String>, html: impl Into<String>) -> Self {
        Self::text(uri, html).with_mime_type(crate::protocol::apps::MIME_TYPE)
    }

    /// Set or override the MIME type
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_wire::content::resource::ResourceContent;
    ///
    /// let json_string = serde_json::json!({"ok": true}).to_string();
    /// let content =
    ///     ResourceContent::text("file:///data.json", json_string).with_mime_type("application/json");
    /// assert_eq!(content.mime_type.as_deref(), Some("application/json"));
    /// ```
    #[must_use]
    pub fn with_mime_type(mut self, mime_type: impl Into<Cow<'static, str>>) -> Self {
        self.mime_type = Some(mime_type.into());
        self
    }

    /// Create a binary resource with base64-encoded data
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_wire::content::resource::ResourceContent;
    ///
    /// let base64_data = String::from("iVBORw==");
    /// let content = ResourceContent::blob("file:///image.png", base64_data, "image/png");
    /// assert_eq!(content.mime_type.as_deref(), Some("image/png"));
    /// ```
    pub fn blob(
        uri: impl Into<String>,
        data: impl Into<String>,
        mime_type: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self::new(uri, ResourcePayload::Blob(data.into())).with_mime_type(mime_type)
    }

    /// Set protocol metadata.
    #[must_use]
    pub fn with_meta(mut self, meta: Map<String, Value>) -> Self {
        self.meta = Some(meta);
        self
    }
}

impl Serialize for ResourceContent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("ResourceContent", 4)?;
        state.serialize_field("uri", &self.uri)?;
        if let Some(mime_type) = &self.mime_type {
            state.serialize_field("mimeType", mime_type)?;
        }
        let (ResourcePayload::Text(payload) | ResourcePayload::Blob(payload)) = &self.payload;
        state.serialize_field(self.payload.key(), payload)?;
        if let Some(meta) = &self.meta {
            state.serialize_field("_meta", meta)?;
        }
        state.end()
    }
}

impl<'de> Deserialize<'de> for ResourceContent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct WireResourceContent {
            uri: String,
            #[serde(default)]
            mime_type: Option<Cow<'static, str>>,
            #[serde(default)]
            text: Option<String>,
            #[serde(default)]
            blob: Option<String>,
            #[serde(rename = "_meta", default)]
            meta: Option<Map<String, Value>>,
        }

        let wire = WireResourceContent::deserialize(deserializer)?;
        let payload = match (wire.text, wire.blob) {
            (Some(text), None) => ResourcePayload::Text(text),
            (None, Some(blob)) => ResourcePayload::Blob(blob),
            _ => {
                return Err(D::Error::custom(
                    "resource content must contain exactly one of `text` or `blob`",
                ));
            }
        };
        Ok(Self {
            uri: wire.uri,
            mime_type: wire.mime_type,
            payload,
            meta: wire.meta,
        })
    }
}

#[cfg(test)]
mod tests;
