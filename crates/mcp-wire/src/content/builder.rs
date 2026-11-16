//! Fluent content builders
//!
//! Provides ergonomic builder API for creating content with annotations

use std::borrow::Cow;

use serde_json::{Map, Value};

use super::annotations::{Annotations, Priority};
use super::types::{
    AudioContent, Icon, ImageContent, ResourceLink, ResourceLinkTag, TextContent, TextTag,
};
use crate::protocol::types::Role;

fn with_annotations_mut(
    annotations: &mut Option<Annotations>,
    update: impl FnOnce(&mut Annotations),
) {
    update(annotations.get_or_insert_with(Annotations::new));
}

/// Generate the fluent annotation setters shared by every content builder.
///
/// Each builder stores `annotations: Option<Annotations>` and exposes the same
/// setters; defining them once here keeps the four builders in lock-step.
macro_rules! annotation_setters {
    () => {
        /// Add audience annotation
        #[must_use]
        pub fn with_audience(mut self, audience: Role) -> Self {
            with_annotations_mut(&mut self.annotations, |annotations| {
                annotations
                    .audience
                    .get_or_insert_with(Vec::new)
                    .push(audience);
            });
            self
        }

        /// Set priority annotation
        #[must_use]
        pub fn with_priority(mut self, priority: Priority) -> Self {
            with_annotations_mut(&mut self.annotations, |annotations| {
                annotations.priority = Some(priority);
            });
            self
        }

        /// Set last modified annotation
        #[must_use]
        pub fn with_last_modified(mut self, timestamp: impl Into<String>) -> Self {
            with_annotations_mut(&mut self.annotations, |annotations| {
                annotations.last_modified = Some(timestamp.into());
            });
            self
        }

        /// Set protocol metadata.
        #[must_use]
        pub fn with_meta(mut self, meta: Map<String, Value>) -> Self {
            self.meta = Some(meta);
            self
        }
    };
}

macro_rules! binary_content_builder {
    (
        $(#[$meta:meta])*
        $builder:ident => $content:ident,
        constructor: $constructor_doc:literal,
        build: $build_doc:literal $(,)?
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone)]
        pub struct $builder {
            data: String,
            mime_type: Cow<'static, str>,
            annotations: Option<Annotations>,
            meta: Option<Map<String, Value>>,
        }

        impl $builder {
            #[doc = $constructor_doc]
            pub fn new(data: impl Into<String>, mime_type: impl Into<Cow<'static, str>>) -> Self {
                Self {
                    data: data.into(),
                    mime_type: mime_type.into(),
                    annotations: None,
                    meta: None,
                }
            }

            annotation_setters!();

            #[doc = $build_doc]
            pub fn build(self) -> $content {
                $content {
                    r#type: Default::default(),
                    data: self.data,
                    mime_type: self.mime_type,
                    annotations: self.annotations,
                    meta: self.meta,
                }
            }
        }
    };
}

/// Builder for text content
#[derive(Debug, Clone)]
pub struct TextContentBuilder {
    text: String,
    annotations: Option<Annotations>,
    meta: Option<Map<String, Value>>,
}

impl TextContentBuilder {
    /// Create new text content builder
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            annotations: None,
            meta: None,
        }
    }

    annotation_setters!();

    /// Build text content
    pub fn build(self) -> TextContent {
        TextContent {
            r#type: TextTag,
            text: self.text,
            annotations: self.annotations,
            meta: self.meta,
        }
    }
}

binary_content_builder!(
    /// Builder for image content
    ImageContentBuilder => ImageContent,
    constructor: "Create new image content builder",
    build: "Build image content",
);

binary_content_builder!(
    /// Builder for audio content
    AudioContentBuilder => AudioContent,
    constructor: "Create new audio content builder",
    build: "Build audio content",
);

/// Builder for resource links
#[derive(Debug, Clone)]
pub struct ResourceLinkBuilder {
    /// Defaults to `uri` at build, so an explicit name never copies it first.
    name: Option<String>,
    uri: String,
    title: Option<String>,
    description: Option<String>,
    mime_type: Option<Cow<'static, str>>,
    icons: Option<Vec<Icon>>,
    annotations: Option<Annotations>,
    size: Option<u64>,
    meta: Option<Map<String, Value>>,
}

impl ResourceLinkBuilder {
    /// Create new resource link builder
    pub fn new(uri: impl Into<String>) -> Self {
        Self {
            name: None,
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

    /// Set the programmatic resource name.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set title
    #[must_use]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the resource description.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the resource MIME type.
    #[must_use]
    pub fn with_mime_type(mut self, mime_type: impl Into<Cow<'static, str>>) -> Self {
        self.mime_type = Some(mime_type.into());
        self
    }

    /// Set the resource icons.
    #[must_use]
    pub fn with_icons(mut self, icons: Vec<Icon>) -> Self {
        self.icons = Some(icons);
        self
    }

    /// Set the raw resource size in bytes.
    #[must_use]
    pub const fn with_size(mut self, size: u64) -> Self {
        self.size = Some(size);
        self
    }

    annotation_setters!();

    /// Build resource link
    pub fn build(self) -> ResourceLink {
        ResourceLink {
            r#type: ResourceLinkTag,
            name: self.name.unwrap_or_else(|| self.uri.clone()),
            uri: self.uri,
            title: self.title,
            description: self.description,
            mime_type: self.mime_type,
            icons: self.icons,
            annotations: self.annotations,
            size: self.size,
            meta: self.meta,
        }
    }
}

/// Convenience function to create text content builder
pub fn text(text: impl Into<String>) -> TextContentBuilder {
    TextContentBuilder::new(text)
}

/// Convenience function to create image content builder
pub fn image(
    data: impl Into<String>,
    mime_type: impl Into<Cow<'static, str>>,
) -> ImageContentBuilder {
    ImageContentBuilder::new(data, mime_type)
}

/// Convenience function to create audio content builder
pub fn audio(
    data: impl Into<String>,
    mime_type: impl Into<Cow<'static, str>>,
) -> AudioContentBuilder {
    AudioContentBuilder::new(data, mime_type)
}

/// Convenience function to create resource link builder
pub fn resource(uri: impl Into<String>) -> ResourceLinkBuilder {
    ResourceLinkBuilder::new(uri)
}

#[cfg(test)]
mod tests;
