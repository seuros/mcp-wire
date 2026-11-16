//! Content types for MCP messages
//!
//! Provides polymorphic content system inspired by Go mcphost

use std::borrow::Cow;

use fluent_uri::component::Scheme;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};

use super::annotations::Annotations;
use super::resource::ResourceContent;
use crate::protocol::version::const_string;

const_string!(
    /// `type` tag of [`TextContent`]: zero-sized, emitted as a literal.
    TextTag = "text"
);
const_string!(
    /// `type` tag of [`ImageContent`]: zero-sized, emitted as a literal.
    ImageTag = "image"
);
const_string!(
    /// `type` tag of [`AudioContent`]: zero-sized, emitted as a literal.
    AudioTag = "audio"
);
const_string!(
    /// `type` tag of [`ResourceLink`]: zero-sized, emitted as a literal.
    ResourceLinkTag = "resource_link"
);
const_string!(
    /// `type` tag of [`EmbeddedResource`]: zero-sized, emitted as a literal.
    EmbeddedResourceTag = "resource"
);

/// Generate the `with_annotations` and `with_meta` setters every content struct shares.
macro_rules! impl_content {
    ($ty:ty) => {
        impl $ty {
            /// Set annotations
            #[must_use]
            pub fn with_annotations(mut self, annotations: Annotations) -> Self {
                self.annotations = Some(annotations);
                self
            }

            /// Set protocol metadata.
            #[must_use]
            pub fn with_meta(mut self, meta: Map<String, Value>) -> Self {
                self.meta = Some(meta);
                self
            }
        }
    };
}

macro_rules! binary_content {
    (
        $(#[$meta:meta])*
        $name:ident,
        tag: $tag:ident,
        data: $data_doc:literal,
        mime: $mime_doc:literal,
        constructor: $constructor_doc:literal $(,)?
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
        pub struct $name {
            /// Content type tag, serialized as a literal.
            #[serde(rename = "type")]
            pub r#type: $tag,

            #[doc = $data_doc]
            pub data: String,

            #[doc = $mime_doc]
            #[serde(rename = "mimeType")]
            pub mime_type: Cow<'static, str>,

            /// Optional annotations
            #[serde(skip_serializing_if = "Option::is_none")]
            pub annotations: Option<Annotations>,

            /// Protocol metadata associated with this content block.
            #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
            pub meta: Option<Map<String, Value>>,
        }

        impl $name {
            /// Convert to JSON, moving owned payload and metadata buffers.
            pub fn into_value(self) -> Value {
                let mut object = content_object($tag::VALUE, self.annotations, self.meta);
                object.insert("data".into(), Value::String(self.data));
                object.insert("mimeType".into(), Value::String(self.mime_type.into_owned()));
                Value::Object(object)
            }

            #[doc = $constructor_doc]
            pub fn new(data: impl Into<String>, mime_type: impl Into<Cow<'static, str>>) -> Self {
                Self {
                    r#type: $tag,
                    data: data.into(),
                    mime_type: mime_type.into(),
                    annotations: None,
                    meta: None,
                }
            }
        }

        impl_content!($name);
    };
}

macro_rules! content_block_setter {
    (
        $(#[$meta:meta])*
        $method:ident($value:ident: $value_type:ty) => $field:ident
    ) => {
        $(#[$meta])*
        #[must_use]
        pub fn $method(mut self, $value: $value_type) -> Self {
            match &mut self {
                Self::Text(content) => content.$field = Some($value),
                Self::Image(content) => content.$field = Some($value),
                Self::Audio(content) => content.$field = Some($value),
                Self::ResourceLink(content) => content.$field = Some($value),
                Self::EmbeddedResource(content) => content.$field = Some($value),
            }
            self
        }
    };
}

macro_rules! deserialize_content_variant {
    ($value:expr, $variant:ident) => {
        serde_json::from_value($value)
            .map(Self::$variant)
            .map_err(D::Error::custom)
    };
}

macro_rules! impl_content_block_from {
    ($($content:ty => $variant:ident),+ $(,)?) => {
        $(
            impl From<$content> for ContentBlock {
                fn from(content: $content) -> Self {
                    Self::$variant(content)
                }
            }
        )+
    };
}

/// Text content
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextContent {
    /// Content type tag, serialized as a literal.
    #[serde(rename = "type")]
    pub r#type: TextTag,

    /// The text
    pub text: String,

    /// Optional annotations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,

    /// Protocol metadata associated with this content block.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<Map<String, Value>>,
}

impl TextContent {
    /// Convert to JSON, moving owned payload and metadata buffers.
    pub fn into_value(self) -> Value {
        let mut object = content_object(TextTag::VALUE, self.annotations, self.meta);
        object.insert("text".into(), Value::String(self.text));
        Value::Object(object)
    }

    /// Create new text content
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            r#type: TextTag,
            text: text.into(),
            annotations: None,
            meta: None,
        }
    }
}

impl_content!(TextContent);

binary_content!(
    /// Image content
    ImageContent,
    tag: ImageTag,
    data: "Base64-encoded image data",
    mime: "MIME type (e.g., \"image/png\")",
    constructor: "Create new image content",
);

binary_content!(
    /// Audio content
    AudioContent,
    tag: AudioTag,
    data: "Base64-encoded audio data",
    mime: "MIME type (e.g., \"audio/wav\")",
    constructor: "Create new audio content",
);

/// Theme for a resource icon.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum IconTheme {
    /// Icon intended for a light background.
    Light,
    /// Icon intended for a dark background.
    Dark,
}

impl IconTheme {
    /// Wire name, as produced by the `lowercase` serde rename.
    const fn as_str(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }
}

/// An [`Icon`] value the schema rejects.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IconError {
    /// `src` is not an RFC 3986 URI.
    #[error("icon src must be a URI, got: {0:?}")]
    InvalidSrc(String),
    /// `src` is a URI outside `http`, `https` and `data`.
    #[error("icon src must be an http, https or data URI, got: {0:?}")]
    UnsafeScheme(String),
    /// A size that is neither `WxH` nor `any`.
    #[error("icon size must be WxH or any, got: {0:?}")]
    InvalidSize(String),
}

/// Schemes an icon `src` may use.
const ICON_SCHEMES: [&Scheme; 3] = [
    Scheme::new_or_panic("http"),
    Scheme::new_or_panic("https"),
    Scheme::new_or_panic("data"),
];

/// An icon `src`: an `http`, `https` or `data` URI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct IconSrc(String);

impl IconSrc {
    /// The URI.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for IconSrc {
    type Error = IconError;

    fn try_from(src: String) -> Result<Self, IconError> {
        let allowed =
            fluent_uri::Uri::parse(src.as_str()).map(|uri| ICON_SCHEMES.contains(&uri.scheme()));
        match allowed {
            Ok(true) => Ok(Self(src)),
            Ok(false) => Err(IconError::UnsafeScheme(src)),
            Err(_) => Err(IconError::InvalidSrc(src)),
        }
    }
}

/// An icon size: `WxH` pixels, or `any` for scalable formats such as SVG.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(try_from = "String")]
pub enum IconSize {
    /// `any`.
    Any,
    /// `WxH`.
    Pixels {
        /// Width in pixels.
        width: u32,
        /// Height in pixels.
        height: u32,
    },
}

impl std::str::FromStr for IconSize {
    type Err = IconError;

    fn from_str(size: &str) -> Result<Self, IconError> {
        let pixels = |digits: &str| {
            digits
                .bytes()
                .all(|byte| byte.is_ascii_digit())
                .then(|| digits.parse().ok())?
        };
        if size == "any" {
            return Ok(Self::Any);
        }
        size.split_once('x')
            .and_then(|(width, height)| {
                Some(Self::Pixels {
                    width: pixels(width)?,
                    height: pixels(height)?,
                })
            })
            .ok_or_else(|| IconError::InvalidSize(size.to_owned()))
    }
}

impl TryFrom<String> for IconSize {
    type Error = IconError;

    fn try_from(size: String) -> Result<Self, IconError> {
        size.parse()
    }
}

impl std::fmt::Display for IconSize {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Any => formatter.write_str("any"),
            Self::Pixels { width, height } => write!(formatter, "{width}x{height}"),
        }
    }
}

impl Serialize for IconSize {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

/// An optionally sized icon associated with a resource link.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", try_from = "ReceivedIcon")]
pub struct Icon {
    /// HTTP(S) or data URI containing the icon.
    pub src: IconSrc,

    /// Optional MIME type override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<Cow<'static, str>>,

    /// Supported dimensions; absent means any size.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sizes: Option<Vec<IconSize>>,

    /// Optional intended theme.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<IconTheme>,
}

/// An icon as received, before `src` and `sizes` are checked.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReceivedIcon {
    src: String,
    mime_type: Option<Cow<'static, str>>,
    sizes: Option<Vec<String>>,
    theme: Option<IconTheme>,
}

impl TryFrom<ReceivedIcon> for Icon {
    type Error = IconError;

    fn try_from(icon: ReceivedIcon) -> Result<Self, IconError> {
        Ok(Self {
            src: icon.src.try_into()?,
            mime_type: icon.mime_type,
            sizes: icon
                .sizes
                .map(|sizes| sizes.into_iter().map(IconSize::try_from).collect())
                .transpose()?,
            theme: icon.theme,
        })
    }
}

/// Decode a received `icons` array, dropping each icon whose `src` or sizes
/// the schema rejects: consumers MUST reject unsafe icon URIs (basic/index.mdx).
///
/// # Errors
///
/// When the value is not an array of icon objects.
pub fn received_icons<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Vec<Icon>>, D::Error> {
    let icons = Option::<Vec<ReceivedIcon>>::deserialize(deserializer)?;
    Ok(icons.map(|icons| {
        icons
            .into_iter()
            .filter_map(|icon| {
                Icon::try_from(icon)
                    .inspect_err(|error| tracing::debug!(%error, "Dropping received icon"))
                    .ok()
            })
            .collect()
    }))
}

impl Icon {
    /// Convert to JSON without copying owned strings.
    pub fn into_value(self) -> Value {
        let mut object = Map::new();
        object.insert("src".into(), Value::String(self.src.0));
        insert_string(&mut object, "mimeType", self.mime_type);
        if let Some(sizes) = self.sizes {
            object.insert(
                "sizes".into(),
                Value::Array(
                    sizes
                        .iter()
                        .map(|size| Value::String(size.to_string()))
                        .collect(),
                ),
            );
        }
        if let Some(theme) = self.theme {
            object.insert("theme".into(), Value::String(theme.as_str().to_owned()));
        }
        Value::Object(object)
    }

    /// Create an icon from its source URI.
    ///
    /// # Errors
    ///
    /// [`IconError::InvalidSrc`] when `src` is not a URI, and
    /// [`IconError::UnsafeScheme`] when it is not `http`, `https` or `data`.
    pub fn new(src: impl Into<String>) -> Result<Self, IconError> {
        Ok(Self {
            src: IconSrc::try_from(src.into())?,
            mime_type: None,
            sizes: None,
            theme: None,
        })
    }

    fluent_setter!(
        /// Set the MIME type override.
        pub fn with_mime_type(mime_type: impl Into<Cow<'static, str>>) => some_into mime_type
    );

    /// Set supported icon sizes.
    #[must_use]
    pub fn with_sizes(mut self, sizes: impl IntoIterator<Item = IconSize>) -> Self {
        self.sizes = Some(sizes.into_iter().collect());
        self
    }

    fluent_setter!(
        /// Set the intended theme.
        pub fn with_theme(theme: IconTheme) => some theme
    );
}

/// Resource link content
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceLink {
    /// Content type tag, serialized as a literal.
    #[serde(rename = "type")]
    pub r#type: ResourceLinkTag,

    /// Programmatic resource name.
    pub name: String,

    /// Resource URI
    pub uri: String,

    /// Optional title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Optional description of the resource.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Optional MIME type of the resource.
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<Cow<'static, str>>,

    /// Optional icons associated with the resource.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::content::types::received_icons"
    )]
    pub icons: Option<Vec<Icon>>,

    /// Optional annotations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,

    /// Size of the raw resource content in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,

    /// Protocol metadata associated with this content block.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<Map<String, Value>>,
}

impl ResourceLink {
    /// Convert to JSON, moving owned payload and metadata buffers.
    pub fn into_value(self) -> Value {
        let mut object = content_object(ResourceLinkTag::VALUE, self.annotations, self.meta);
        object.insert("name".into(), Value::String(self.name));
        object.insert("uri".into(), Value::String(self.uri));
        insert_string(&mut object, "title", self.title);
        insert_string(&mut object, "description", self.description);
        insert_string(&mut object, "mimeType", self.mime_type);
        if let Some(icons) = self.icons {
            object.insert(
                "icons".into(),
                Value::Array(icons.into_iter().map(Icon::into_value).collect()),
            );
        }
        if let Some(size) = self.size {
            object.insert("size".into(), Value::from(size));
        }
        Value::Object(object)
    }

    /// Create new resource link
    pub fn new(uri: impl Into<String>) -> Self {
        let uri = uri.into();
        Self::named(uri.clone(), uri)
    }

    /// Create a resource link with an explicit programmatic name.
    pub fn named(name: impl Into<String>, uri: impl Into<String>) -> Self {
        Self {
            r#type: ResourceLinkTag,
            name: name.into(),
            uri: uri.into(),
            title: None,
            description: None,
            mime_type: None,
            icons: None,
            annotations: None,
            size: None,
            meta: None,
        }
    }

    fluent_setter!(
        /// Set the programmatic resource name.
        pub fn with_name(name: impl Into<String>) => into name
    );

    fluent_setter!(
        /// Set title
        pub fn with_title(title: impl Into<String>) => some_into title
    );

    fluent_setter!(
        /// Set the resource description.
        pub fn with_description(description: impl Into<String>) => some_into description
    );

    fluent_setter!(
        /// Set the resource MIME type.
        pub fn with_mime_type(mime_type: impl Into<Cow<'static, str>>) => some_into mime_type
    );

    fluent_setter!(
        /// Set the resource icons.
        pub fn with_icons(icons: Vec<Icon>) => some icons
    );

    fluent_setter!(
        /// Set the raw resource size in bytes.
        pub fn with_size(size: u64) => some size
    );
}

impl_content!(ResourceLink);

/// Embedded resource content block
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EmbeddedResource {
    /// Content type tag, serialized as a literal.
    #[serde(rename = "type")]
    pub r#type: EmbeddedResourceTag,

    /// Embedded resource contents
    pub resource: ResourceContent,

    /// Optional annotations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,

    /// Protocol metadata associated with this content block.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<Map<String, Value>>,
}

impl EmbeddedResource {
    /// Convert to JSON, moving owned payload and metadata buffers.
    pub fn into_value(self) -> Value {
        let mut object = content_object(EmbeddedResourceTag::VALUE, self.annotations, self.meta);
        object.insert("resource".into(), self.resource.into_value());
        Value::Object(object)
    }

    /// Create new embedded resource content
    pub const fn new(resource: ResourceContent) -> Self {
        Self {
            r#type: EmbeddedResourceTag,
            resource,
            annotations: None,
            meta: None,
        }
    }
}

impl From<ResourceContent> for EmbeddedResource {
    fn from(resource: ResourceContent) -> Self {
        Self::new(resource)
    }
}

impl_content!(EmbeddedResource);

/// Canonical MCP content-block wire representation.
///
/// This type is shared by prompt messages and tool results so every supported
/// content block has identical serialization behavior in both protocol paths.
#[derive(Debug, Clone, PartialEq)]
pub enum ContentBlock {
    /// Text content.
    Text(TextContent),
    /// Image content.
    Image(ImageContent),
    /// Audio content.
    Audio(AudioContent),
    /// A link to a readable resource.
    ResourceLink(ResourceLink),
    /// Resource contents embedded directly in the message.
    EmbeddedResource(EmbeddedResource),
}

impl ContentBlock {
    /// Convert to JSON, moving owned payload and metadata buffers.
    pub fn into_value(self) -> Value {
        match self {
            Self::Text(content) => content.into_value(),
            Self::Image(content) => content.into_value(),
            Self::Audio(content) => content.into_value(),
            Self::ResourceLink(content) => content.into_value(),
            Self::EmbeddedResource(content) => content.into_value(),
        }
    }

    /// Create a text content block.
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text(TextContent::new(text))
    }

    /// Create an image content block.
    pub fn image(data: impl Into<String>, mime_type: impl Into<Cow<'static, str>>) -> Self {
        Self::Image(ImageContent::new(data, mime_type))
    }

    /// Create an audio content block.
    pub fn audio(data: impl Into<String>, mime_type: impl Into<Cow<'static, str>>) -> Self {
        Self::Audio(AudioContent::new(data, mime_type))
    }

    /// Create a resource-link content block.
    pub fn resource(uri: impl Into<String>) -> Self {
        Self::ResourceLink(ResourceLink::new(uri))
    }

    /// Create a named resource-link content block.
    pub fn resource_named(name: impl Into<String>, uri: impl Into<String>) -> Self {
        Self::ResourceLink(ResourceLink::named(name, uri))
    }

    /// Create an embedded-resource content block.
    pub const fn embedded_resource(resource: ResourceContent) -> Self {
        Self::EmbeddedResource(EmbeddedResource::new(resource))
    }

    content_block_setter!(
        /// Set annotations on this content block.
        with_annotations(annotations: Annotations) => annotations
    );

    content_block_setter!(
        /// Set protocol metadata on this content block.
        with_meta(meta: Map<String, Value>) => meta
    );
}

impl Serialize for ContentBlock {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Text(content) => content.serialize(serializer),
            Self::Image(content) => content.serialize(serializer),
            Self::Audio(content) => content.serialize(serializer),
            Self::ResourceLink(content) => content.serialize(serializer),
            Self::EmbeddedResource(content) => content.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for ContentBlock {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        let content_type = value
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| D::Error::custom("content block requires a string `type` field"))?;

        match content_type {
            TextTag::VALUE => deserialize_content_variant!(value, Text),
            ImageTag::VALUE => deserialize_content_variant!(value, Image),
            AudioTag::VALUE => deserialize_content_variant!(value, Audio),
            ResourceLinkTag::VALUE => deserialize_content_variant!(value, ResourceLink),
            EmbeddedResourceTag::VALUE => deserialize_content_variant!(value, EmbeddedResource),
            other => Err(D::Error::custom(format!(
                "unsupported content block type `{other}`"
            ))),
        }
    }
}

impl_content_block_from!(
    TextContent => Text,
    ImageContent => Image,
    AudioContent => Audio,
    ResourceLink => ResourceLink,
    EmbeddedResource => EmbeddedResource,
);

impl From<ResourceContent> for ContentBlock {
    fn from(resource: ResourceContent) -> Self {
        Self::embedded_resource(resource)
    }
}

/// Insert an optional string, moving an owned buffer or materializing a
/// borrowed `Cow` (the JSON tree owns its strings).
pub fn insert_string(object: &mut Map<String, Value>, key: &str, value: Option<impl Into<String>>) {
    if let Some(value) = value {
        object.insert(key.into(), Value::String(value.into()));
    }
}

/// Insert an optional JSON object, moving its map.
pub fn insert_object(
    object: &mut Map<String, Value>,
    key: &str,
    value: Option<Map<String, Value>>,
) {
    if let Some(value) = value {
        object.insert(key.into(), Value::Object(value));
    }
}

/// Start a content-block object. `Value` owns its strings, so the literal tag
/// is materialized here, once, like every key of the JSON tree.
pub(crate) fn content_object(
    content_type: &'static str,
    annotations: Option<Annotations>,
    meta: Option<Map<String, Value>>,
) -> Map<String, Value> {
    let mut object = Map::new();
    object.insert("type".into(), Value::String(content_type.to_owned()));
    if let Some(annotations) = annotations {
        object.insert("annotations".into(), annotations.into_value());
    }
    insert_object(&mut object, "_meta", meta);
    object
}

#[cfg(test)]
mod tests;
