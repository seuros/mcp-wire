//! MCP Apps extension (`io.modelcontextprotocol/ui`, ext-apps 2026-01-26)

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use super::capabilities::EmptyObject;
use crate::content::types::insert_string;

/// Extension identifier negotiated in `capabilities.extensions`.
pub const EXTENSION_ID: &str = "io.modelcontextprotocol/ui";

/// MIME type of MCP Apps HTML views.
pub const MIME_TYPE: &str = "text/html;profile=mcp-app";

/// URI scheme prefix of MCP Apps resources.
pub const URI_SCHEME: &str = "ui://";

const HTTP_SCHEMES: &[&str] = &["http", "https"];
const CONNECT_SCHEMES: &[&str] = &["http", "https", "ws", "wss"];

/// Extension settings a server advertises in `capabilities.extensions`.
pub fn extension_settings() -> Map<String, Value> {
    crate::utils::json_object([("mimeTypes".to_string(), json!([MIME_TYPE]))])
}

/// Whether `uri` is a printable-ASCII `ui://` URI. `const` so `#[mcp_tool]` checks it at compile time.
pub const fn is_ui_uri(uri: &str) -> bool {
    let bytes = uri.as_bytes();
    let scheme = URI_SCHEME.as_bytes();
    if bytes.len() <= scheme.len() {
        return false;
    }

    let mut i = 0;
    while i < bytes.len() {
        let valid = if i < scheme.len() {
            bytes[i] == scheme[i]
        } else {
            is_uri_byte(bytes[i])
        };
        if !valid {
            return false;
        }
        i += 1;
    }
    true
}

const fn is_uri_byte(byte: u8) -> bool {
    matches!(byte, 0x21..=0x7e)
}

/// Whether tool `_meta.ui.visibility` omits `model`.
pub fn is_app_only(meta: &Map<String, Value>) -> bool {
    meta.get("ui")
        .and_then(|ui| ui.get("visibility"))
        .and_then(Value::as_array)
        .is_some_and(|callers| !callers.iter().any(|caller| caller == "model"))
}

/// Invalid MCP Apps metadata.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AppsError {
    /// Resource URI is not a `ui://` URI.
    #[error("MCP Apps resource URI must be a ui:// URI, got: {0:?}")]
    InvalidUri(String),

    /// CSP origin has an unsupported scheme or malformed host.
    #[error("ui csp {directive} entries must be {expected} origins, got: {origin:?}")]
    InvalidOrigin {
        /// CSP directive holding the origin.
        directive: &'static str,
        /// Accepted origin schemes.
        expected: &'static str,
        /// Rejected origin.
        origin: String,
    },
}

/// Caller allowed to invoke a tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UiVisibility {
    /// The model.
    Model,
    /// Views rendered from this server.
    App,
}

impl UiVisibility {
    /// Wire name, as produced by the `lowercase` serde rename.
    const fn as_str(self) -> &'static str {
        match self {
            Self::Model => "model",
            Self::App => "app",
        }
    }
}

/// Tool-level `_meta.ui`: the view rendering the tool and who may call it.
///
/// App-only tools (visibility without `model`) are hidden from clients that
/// cannot render MCP Apps.
///
/// ```rust
/// use mcp_wire::protocol::apps::{ToolUi, UiVisibility};
///
/// let meta = ToolUi::new("ui://weather/dashboard")?
///     .with_visibility([UiVisibility::App])
///     .into_meta();
/// assert_eq!(meta["ui"]["resourceUri"], "ui://weather/dashboard");
///
/// let model_only = ToolUi::visibility_only([UiVisibility::Model]).into_meta();
/// assert_eq!(model_only["ui"]["visibility"], serde_json::json!(["model"]));
/// # Ok::<(), mcp_wire::protocol::apps::AppsError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolUi {
    #[serde(skip_serializing_if = "Option::is_none")]
    resource_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    visibility: Option<Vec<UiVisibility>>,
}

impl ToolUi {
    /// Link to the `ui://` view at `resource_uri`.
    ///
    /// # Errors
    ///
    /// [`AppsError::InvalidUri`] when `resource_uri` is not a printable-ASCII
    /// `ui://` URI (see [`is_ui_uri`]).
    pub fn new(resource_uri: impl Into<String>) -> Result<Self, AppsError> {
        let resource_uri = resource_uri.into();
        if !is_ui_uri(&resource_uri) {
            return Err(AppsError::InvalidUri(resource_uri));
        }

        Ok(Self {
            resource_uri: Some(resource_uri),
            visibility: None,
        })
    }

    /// Restrict callers without linking a view.
    pub fn visibility_only(visibility: impl IntoIterator<Item = UiVisibility>) -> Self {
        Self {
            resource_uri: None,
            visibility: None,
        }
        .with_visibility(visibility)
    }

    /// Restrict who may call the tool; hosts default to model and app.
    #[must_use]
    pub fn with_visibility(mut self, visibility: impl IntoIterator<Item = UiVisibility>) -> Self {
        self.visibility = Some(visibility.into_iter().collect());
        self
    }

    /// Linked view URI.
    pub fn resource_uri(&self) -> Option<&str> {
        self.resource_uri.as_deref()
    }

    /// Explicit visibility.
    pub fn visibility(&self) -> Option<&[UiVisibility]> {
        self.visibility.as_deref()
    }

    /// `_meta` object for `Tool::meta` (mcp-host).
    pub fn into_meta(self) -> Map<String, Value> {
        let mut ui = Map::new();
        insert_string(&mut ui, "resourceUri", self.resource_uri);
        if let Some(visibility) = self.visibility {
            ui.insert(
                "visibility".into(),
                visibility
                    .into_iter()
                    .map(|caller| Value::String(caller.as_str().to_owned()))
                    .collect(),
            );
        }
        ui_meta(ui)
    }
}

/// Resource-level `_meta.ui`: sandbox policy for a `ui://` view.
///
/// Content-level metadata wins over the `resources/list` entry.
/// [`into_meta`](Self::into_meta) validates; serializing directly does not.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiResourceMeta {
    /// Origins the view may reach; deny-by-default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub csp: Option<UiCsp>,

    /// Browser permissions requested for the view.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<UiPermissions>,

    /// Dedicated sandbox origin.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,

    /// Whether the host should draw a border around the view.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefers_border: Option<bool>,
}

impl UiResourceMeta {
    /// Validated `_meta` object for resource contents or listings.
    ///
    /// # Errors
    ///
    /// [`AppsError::InvalidOrigin`] when `csp` holds an origin that
    /// [`UiCsp::validate`] rejects.
    pub fn into_meta(self) -> Result<Map<String, Value>, AppsError> {
        let mut ui = Map::new();
        if let Some(csp) = self.csp {
            csp.validate()?;
            ui.insert("csp".into(), csp.into_value());
        }
        if let Some(permissions) = self.permissions {
            ui.insert("permissions".into(), permissions.into_value());
        }
        insert_string(&mut ui, "domain", self.domain);
        if let Some(prefers_border) = self.prefers_border {
            ui.insert("prefersBorder".into(), Value::Bool(prefers_border));
        }
        Ok(ui_meta(ui))
    }
}

/// Content Security Policy origins for a view.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiCsp {
    /// `connect-src` origins (`http(s)://` or `ws(s)://`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connect_domains: Option<Vec<String>>,

    /// Script, style, image, font, and media origins.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_domains: Option<Vec<String>>,

    /// `frame-src` origins.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_domains: Option<Vec<String>>,

    /// `base-uri` origins.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_uri_domains: Option<Vec<String>>,
}

impl UiCsp {
    /// Reject origins with unsupported schemes or malformed hosts.
    ///
    /// # Errors
    ///
    /// [`AppsError::InvalidOrigin`] for the first origin whose scheme is not
    /// allowed for its directive (`ws(s)://` only in `connectDomains`), whose
    /// host is empty or starts with `/`, or which contains a non-printable
    /// byte, a quote, `<`, `>`, `;`, or `,`.
    pub fn validate(&self) -> Result<(), AppsError> {
        validate_origins(
            "connectDomains",
            self.connect_domains.as_deref(),
            CONNECT_SCHEMES,
        )?;
        validate_origins(
            "resourceDomains",
            self.resource_domains.as_deref(),
            HTTP_SCHEMES,
        )?;
        validate_origins("frameDomains", self.frame_domains.as_deref(), HTTP_SCHEMES)?;
        validate_origins(
            "baseUriDomains",
            self.base_uri_domains.as_deref(),
            HTTP_SCHEMES,
        )
    }

    /// Convert to JSON, moving the origin lists.
    fn into_value(self) -> Value {
        let mut object = Map::new();
        for (key, origins) in [
            ("connectDomains", self.connect_domains),
            ("resourceDomains", self.resource_domains),
            ("frameDomains", self.frame_domains),
            ("baseUriDomains", self.base_uri_domains),
        ] {
            if let Some(origins) = origins {
                object.insert(key.into(), origins.into_iter().map(Value::String).collect());
            }
        }
        Value::Object(object)
    }
}

/// Browser permissions requested for a view.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiPermissions {
    /// Camera access.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera: Option<EmptyObject>,

    /// Microphone access.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub microphone: Option<EmptyObject>,

    /// Geolocation access.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geolocation: Option<EmptyObject>,

    /// Clipboard write access.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clipboard_write: Option<EmptyObject>,
}

impl UiPermissions {
    /// Convert to JSON: each granted permission is an empty object.
    fn into_value(self) -> Value {
        let mut object = Map::new();
        for (key, permission) in [
            ("camera", self.camera),
            ("microphone", self.microphone),
            ("geolocation", self.geolocation),
            ("clipboardWrite", self.clipboard_write),
        ] {
            if permission.is_some() {
                object.insert(key.into(), Value::Object(Map::new()));
            }
        }
        Value::Object(object)
    }
}

// `;` and `,` separate CSP directives and sources.
fn validate_origins(
    directive: &'static str,
    origins: Option<&[String]>,
    schemes: &[&str],
) -> Result<(), AppsError> {
    for origin in origins.unwrap_or_default() {
        let valid = origin.split_once("://").is_some_and(|(scheme, host)| {
            schemes.contains(&scheme)
                && !host.is_empty()
                && !host.starts_with('/')
                && host
                    .bytes()
                    .all(|byte| is_uri_byte(byte) && !b"\"'<>;,".contains(&byte))
        });
        if !valid {
            let expected = if schemes == CONNECT_SCHEMES {
                "http(s):// or ws(s)://"
            } else {
                "http(s)://"
            };
            return Err(AppsError::InvalidOrigin {
                directive,
                expected,
                origin: origin.clone(),
            });
        }
    }

    Ok(())
}

fn ui_meta(ui: Map<String, Value>) -> Map<String, Value> {
    crate::utils::json_object([("ui".to_string(), Value::Object(ui))])
}

#[cfg(test)]
mod tests;
