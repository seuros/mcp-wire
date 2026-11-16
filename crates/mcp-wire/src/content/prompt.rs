//! Prompt message types for MCP prompts/get responses
//!
//! Per MCP spec, prompt responses have `messages` with `role` + `content`

use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use super::resource::ResourceContent;
use super::types::{ContentBlock, ResourceLink};

/// Content in a prompt message.
///
/// Prompt messages use the same complete MCP content-block union as tool
/// results, including audio and resource links.
pub type PromptContent = ContentBlock;

/// A single message in a prompt response
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PromptMessage {
    /// Message role (user or assistant)
    pub role: PromptRole,

    /// Message content
    pub content: PromptContent,
}

/// Role in a prompt conversation
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PromptRole {
    /// User message
    User,

    /// Assistant message
    Assistant,
}

impl PromptRole {
    /// Wire name, as produced by the `lowercase` serde rename.
    const fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
        }
    }
}

macro_rules! prompt_message_pair {
    (
        $(#[$user_meta:meta])*
        $user:ident,
        $(#[$assistant_meta:meta])*
        $assistant:ident,
        ($($arg:ident: $arg_type:ty),* $(,)?) => $content:expr $(,)?
    ) => {
        $(#[$user_meta])*
        pub fn $user($($arg: $arg_type),*) -> Self {
            Self::new(PromptRole::User, $content)
        }

        $(#[$assistant_meta])*
        pub fn $assistant($($arg: $arg_type),*) -> Self {
            Self::new(PromptRole::Assistant, $content)
        }
    };
}

impl PromptMessage {
    /// Convert to JSON, moving this message's content payload.
    pub fn into_value(self) -> serde_json::Value {
        serde_json::Value::Object(crate::utils::json_object([
            (
                "role".into(),
                serde_json::Value::String(self.role.as_str().to_owned()),
            ),
            ("content".into(), self.content.into_value()),
        ]))
    }

    /// Create a prompt message from any MCP content block.
    pub fn new(role: PromptRole, content: impl Into<ContentBlock>) -> Self {
        Self {
            role,
            content: content.into(),
        }
    }

    prompt_message_pair!(
        /// Create a user message with text content
        user,
        /// Create an assistant message with text content
        assistant,
        (text: impl Into<String>) => ContentBlock::text(text),
    );

    prompt_message_pair!(
        /// Create a user message with image content
        user_image,
        /// Create an assistant message with image content
        assistant_image,
        (data: impl Into<String>, mime_type: impl Into<Cow<'static, str>>) =>
            ContentBlock::image(data, mime_type),
    );

    prompt_message_pair!(
        /// Create a user message with audio content.
        user_audio,
        /// Create an assistant message with audio content.
        assistant_audio,
        (data: impl Into<String>, mime_type: impl Into<Cow<'static, str>>) =>
            ContentBlock::audio(data, mime_type),
    );

    prompt_message_pair!(
        /// Create a user message with an embedded resource.
        user_resource,
        /// Create an assistant message with an embedded resource.
        assistant_resource,
        (resource: ResourceContent) => ContentBlock::embedded_resource(resource),
    );

    prompt_message_pair!(
        /// Create a user message with a resource link.
        user_resource_link,
        /// Create an assistant message with a resource link.
        assistant_resource_link,
        (resource: ResourceLink) => resource,
    );
}

#[cfg(test)]
mod tests;
