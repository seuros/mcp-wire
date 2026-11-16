use std::borrow::Cow;
use std::fmt;

use serde::de::value::{MapAccessDeserializer, SeqAccessDeserializer};
use serde::de::{MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};
use thiserror::Error;

use crate::content::annotations::{Annotations, Priority};
use crate::content::types::{
    AudioContent, AudioTag, ContentBlock, Icon, ImageContent, ImageTag, TextContent, TextTag,
    content_object, insert_object, insert_string,
};
use crate::tools::{ToolDefinitionError, ToolInfo, validate_tool_schemas};

use super::{TaskMetadata, ToolAnnotations, ToolExecution};

/// A root directory or file the client lets the server operate on.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    /// `file://` URI of the root (client/roots.mdx:110-111).
    pub uri: String,
    /// Optional human-readable name for the root
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Additional root metadata
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<Map<String, Value>>,
}

/// Result of listing roots
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListRootsResult {
    /// List of roots provided by the client
    pub roots: Vec<Root>,
}

/// Notification that the client's root list has changed
///
/// Servers should re-request the roots list when receiving this
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootsListChangedNotification {}

/// Role in a conversation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// User message
    User,
    /// Assistant message
    Assistant,
}

impl Role {
    /// Wire name, as produced by the `lowercase` serde rename.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
        }
    }
}

/// A message in a sampling conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SamplingMessage {
    /// Role of the message sender
    pub role: Role,

    /// Message content
    pub content: SamplingMessageContent,

    /// Additional message metadata
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<Map<String, Value>>,
}

impl SamplingMessage {
    /// Convert to JSON, moving content payloads.
    fn into_value(self) -> Result<Value, serde_json::Error> {
        let mut object = crate::utils::json_object([
            ("role".into(), serde_json::to_value(self.role)?),
            ("content".into(), self.content.into_value()),
        ]);
        insert_object(&mut object, "_meta", self.meta);
        Ok(Value::Object(object))
    }
}

/// Content in a sampling message, either a single block or multiple blocks.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum SamplingMessageContent {
    /// Single content block
    Single(SamplingContent),
    /// Multiple content blocks
    Multiple(Vec<SamplingContent>),
}

impl SamplingMessageContent {
    /// The content blocks, one for [`Self::Single`].
    pub fn blocks(&self) -> &[SamplingContent] {
        match self {
            Self::Single(content) => std::slice::from_ref(content),
            Self::Multiple(contents) => contents,
        }
    }

    /// The text blocks, joined by newlines; images and audio are skipped.
    pub fn text(&self) -> String {
        let mut texts = self.blocks().iter().filter_map(|block| match block {
            SamplingContent::Text { text, .. } => Some(text.as_str()),
            _ => None,
        });
        let mut joined = texts.next().map_or_default(str::to_owned);
        for text in texts {
            joined.push('\n');
            joined.push_str(text);
        }
        joined
    }

    /// [`Self::text`], moving the text of a single text block.
    pub fn into_text(self) -> String {
        match self {
            Self::Single(SamplingContent::Text { text, .. }) => text,
            content => content.text(),
        }
    }

    fn into_value(self) -> Value {
        match self {
            Self::Single(content) => content.into_value(),
            Self::Multiple(contents) => Value::Array(
                contents
                    .into_iter()
                    .map(SamplingContent::into_value)
                    .collect(),
            ),
        }
    }
}

/// Decodes a block from an object and a list from an array, moving the input's
/// strings; `#[serde(untagged)]` would copy each one out of a buffered tree.
impl<'de> Deserialize<'de> for SamplingMessageContent {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ContentVisitor;

        impl<'de> Visitor<'de> for ContentVisitor {
            type Value = SamplingMessageContent;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a sampling content block or an array of them")
            }

            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
                SamplingContent::deserialize(MapAccessDeserializer::new(map))
                    .map(SamplingMessageContent::Single)
            }

            fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<Self::Value, A::Error> {
                Vec::deserialize(SeqAccessDeserializer::new(seq))
                    .map(SamplingMessageContent::Multiple)
            }
        }

        deserializer.deserialize_any(ContentVisitor)
    }
}

impl From<SamplingContent> for SamplingMessageContent {
    fn from(content: SamplingContent) -> Self {
        Self::Single(content)
    }
}

impl From<Vec<SamplingContent>> for SamplingMessageContent {
    fn from(content: Vec<SamplingContent>) -> Self {
        Self::Multiple(content)
    }
}

/// Content in a sampling message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SamplingContent {
    /// Text content
    #[serde(rename = "text")]
    Text {
        /// Text message body
        text: String,
        /// Optional client-facing annotations
        #[serde(skip_serializing_if = "Option::is_none")]
        annotations: Option<Annotations>,
        /// Additional content metadata
        #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
        meta: Option<Map<String, Value>>,
    },

    /// Image content
    #[serde(rename = "image")]
    Image {
        /// Base64-encoded image bytes
        data: String,
        /// MIME type for the image data
        #[serde(rename = "mimeType")]
        mime_type: Cow<'static, str>,
        /// Optional client-facing annotations
        #[serde(skip_serializing_if = "Option::is_none")]
        annotations: Option<Annotations>,
        /// Additional content metadata
        #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
        meta: Option<Map<String, Value>>,
    },

    /// Audio content
    #[serde(rename = "audio")]
    Audio {
        /// Base64-encoded audio bytes
        data: String,
        /// MIME type for the audio data
        #[serde(rename = "mimeType")]
        mime_type: Cow<'static, str>,
        /// Optional client-facing annotations
        #[serde(skip_serializing_if = "Option::is_none")]
        annotations: Option<Annotations>,
        /// Additional content metadata
        #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
        meta: Option<Map<String, Value>>,
    },

    /// Tool use requested by the model
    #[serde(rename = "tool_use")]
    ToolUse {
        /// Tool use ID
        id: String,
        /// Tool name
        name: String,
        /// Tool input
        input: Map<String, Value>,
        /// Additional tool-use metadata
        #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
        meta: Option<Map<String, Value>>,
    },

    /// Tool result returned to the model
    #[serde(rename = "tool_result")]
    ToolResult {
        /// Matching tool use ID
        #[serde(rename = "toolUseId")]
        tool_use_id: String,
        /// Tool result content
        content: Vec<ContentBlock>,
        /// Structured tool result for machine consumption
        #[serde(rename = "structuredContent", skip_serializing_if = "Option::is_none")]
        structured_content: Option<Map<String, Value>>,
        /// Whether the tool execution failed
        #[serde(rename = "isError", skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
        /// Additional tool-result metadata
        #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
        meta: Option<Map<String, Value>>,
    },
}

impl SamplingContent {
    /// Create a text sampling content block.
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text {
            text: text.into(),
            annotations: None,
            meta: None,
        }
    }

    /// Create an image sampling content block.
    pub fn image(data: impl Into<String>, mime_type: impl Into<Cow<'static, str>>) -> Self {
        Self::Image {
            data: data.into(),
            mime_type: mime_type.into(),
            annotations: None,
            meta: None,
        }
    }

    /// Create an audio sampling content block.
    pub fn audio(data: impl Into<String>, mime_type: impl Into<Cow<'static, str>>) -> Self {
        Self::Audio {
            data: data.into(),
            mime_type: mime_type.into(),
            annotations: None,
            meta: None,
        }
    }

    /// Convert to JSON, moving payloads.
    fn into_value(self) -> Value {
        match self {
            Self::Text {
                text,
                annotations,
                meta,
            } => TextContent {
                r#type: TextTag,
                text,
                annotations,
                meta,
            }
            .into_value(),
            Self::Image {
                data,
                mime_type,
                annotations,
                meta,
            } => ImageContent {
                r#type: ImageTag,
                data,
                mime_type,
                annotations,
                meta,
            }
            .into_value(),
            Self::Audio {
                data,
                mime_type,
                annotations,
                meta,
            } => AudioContent {
                r#type: AudioTag,
                data,
                mime_type,
                annotations,
                meta,
            }
            .into_value(),
            Self::ToolUse {
                id,
                name,
                input,
                meta,
            } => {
                let mut object = content_object("tool_use", None, meta);
                object.insert("id".into(), Value::String(id));
                object.insert("name".into(), Value::String(name));
                object.insert("input".into(), Value::Object(input));
                Value::Object(object)
            }
            Self::ToolResult {
                tool_use_id,
                content,
                structured_content,
                is_error,
                meta,
            } => {
                let mut object = content_object("tool_result", None, meta);
                object.insert("toolUseId".into(), Value::String(tool_use_id));
                object.insert(
                    "content".into(),
                    Value::Array(content.into_iter().map(ContentBlock::into_value).collect()),
                );
                insert_object(&mut object, "structuredContent", structured_content);
                if let Some(is_error) = is_error {
                    object.insert("isError".into(), Value::Bool(is_error));
                }
                Value::Object(object)
            }
        }
    }
}

/// Specifies how much context should be included in sampling requests
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContextInclusion {
    /// No context
    #[default]
    None,
    /// Context from this server only
    ThisServer,
    /// Context from all connected servers
    AllServers,
}

optional_field_struct! {
    /// Model hint for LLM selection
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ModelHint {
        /// Preferred model name (e.g., "claude-3-sonnet", "gpt-4")
        name: String
    }
}

/// Model selection preferences
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelPreferences {
    /// Model name hints
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hints: Option<Vec<ModelHint>>,
    /// Cost priority (higher means prefer cheaper models)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_priority: Option<Priority>,
    /// Speed priority (higher means prefer faster models)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed_priority: Option<Priority>,
    /// Intelligence priority (higher means prefer smarter models)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intelligence_priority: Option<Priority>,
}

/// Tool definition available during sampling.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SamplingTool {
    /// Tool name
    pub name: String,
    /// Human-readable display title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Tool description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Tool input schema
    pub input_schema: Value,
    /// Sized icons for client display
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::content::types::received_icons"
    )]
    pub icons: Option<Vec<Icon>>,
    /// Tool output schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<Value>,
    /// Execution metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution: Option<ToolExecution>,
    /// Behavioral annotations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<ToolAnnotations>,
    /// Additional tool metadata
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<Map<String, Value>>,
}

impl SamplingTool {
    /// Convert to JSON as the identical `tools/list` entry, moving schemas and strings.
    fn into_value(self) -> Result<Value, serde_json::Error> {
        ToolInfo {
            name: self.name,
            title: self.title,
            description: self.description,
            icons: self.icons,
            input_schema: self.input_schema,
            output_schema: self.output_schema,
            execution: self.execution,
            annotations: self.annotations,
            meta: self.meta,
        }
        .into_value()
    }
}

/// Tool choice mode for sampling.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ToolChoiceMode {
    /// Model may decide whether to use tools
    #[default]
    Auto,
    /// Model must use at least one tool
    Required,
    /// Model must not use tools
    None,
}

impl ToolChoiceMode {
    #[expect(
        clippy::trivially_copy_pass_by_ref,
        reason = "serde `skip_serializing_if` passes the field by reference"
    )]
    fn is_auto(&self) -> bool {
        *self == Self::Auto
    }
}

/// Tool choice configuration for sampling.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolChoice {
    /// Tool choice mode, defaulting to automatic selection
    #[serde(default, skip_serializing_if = "ToolChoiceMode::is_auto")]
    pub mode: ToolChoiceMode,
}

/// Why [`CreateMessageParams::validate`] rejected a request; indexes count messages.
#[derive(Debug, Error)]
pub enum SamplingError {
    /// A tool's input or output schema is not an object-root schema.
    #[error(transparent)]
    Tool(#[from] ToolDefinitionError),
    /// A user message holds a tool use.
    #[error("message {0}: tool_use content belongs in an assistant message")]
    ToolUseRole(usize),
    /// An assistant message holds a tool result.
    #[error("message {0}: tool_result content belongs in a user message")]
    ToolResultRole(usize),
    /// A message mixes tool results with other content.
    #[error("message {0}: a message with tool results must hold only tool results")]
    MixedToolResults(usize),
    /// An assistant message's tool uses are not answered, one result each, by the next message.
    #[error("message {0}: the next message must answer each tool use with one tool result")]
    UnansweredToolUse(usize),
    /// A message's tool results answer no tool use of the message before it.
    #[error("message {0}: tool results must answer tool uses of the previous message")]
    UnmatchedToolResult(usize),
}

/// Parameters for a `sampling/createMessage` request.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMessageParams {
    /// Conversation history
    pub messages: Vec<SamplingMessage>,
    /// Model selection preferences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_preferences: Option<ModelPreferences>,
    /// System prompt to use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    /// How much context to include
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_context: Option<ContextInclusion>,
    /// Temperature (0.0-1.0+)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    /// Maximum tokens to generate
    pub max_tokens: u32,
    /// Stop sequences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    /// Provider-specific sampling metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Map<String, Value>>,
    /// Tools available to the model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<SamplingTool>>,
    /// Tool choice configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    /// Task augmentation requested for this sampling operation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<TaskMetadata>,
    /// Additional request metadata
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<Map<String, Value>>,
}

impl CreateMessageParams {
    /// Whether these params require `ClientCapabilities.sampling.context`.
    pub const fn requires_context_capability(&self) -> bool {
        matches!(
            self.include_context,
            Some(ContextInclusion::ThisServer | ContextInclusion::AllServers)
        )
    }

    /// Whether these params require `ClientCapabilities.sampling.tools`.
    pub const fn requires_tools_capability(&self) -> bool {
        self.tools.is_some() || self.tool_choice.is_some()
    }

    /// Check the tools and the tool-use rules of the conversation.
    ///
    /// Tool uses belong in assistant messages and tool results in user
    /// messages (client/sampling.mdx:460-461); a message with tool results
    /// holds nothing else (:372); each assistant message's tool uses are
    /// answered, one result per `id`, by the next message (:430), and every
    /// result answers a use of the message before it (schema.ts
    /// `ToolResultContent.toolUseId`).
    ///
    /// # Errors
    ///
    /// The first [`SamplingError`] found, tools first.
    pub fn validate(&self) -> Result<(), SamplingError> {
        for tool in self.tools.iter().flatten() {
            validate_tool_schemas(&tool.name, &tool.input_schema, tool.output_schema.as_ref())?;
        }
        // Sorted IDs of the tool uses the previous message left to answer.
        let mut open: Vec<&str> = Vec::new();
        for (index, message) in self.messages.iter().enumerate() {
            let (mut uses, mut results, mut other) = (Vec::new(), Vec::new(), false);
            for block in message.content.blocks() {
                match block {
                    SamplingContent::ToolUse { id, .. } => uses.push(id.as_str()),
                    SamplingContent::ToolResult { tool_use_id, .. } => {
                        results.push(tool_use_id.as_str());
                    }
                    _ => other = true,
                }
            }
            match message.role {
                Role::User if !uses.is_empty() => return Err(SamplingError::ToolUseRole(index)),
                Role::Assistant if !results.is_empty() => {
                    return Err(SamplingError::ToolResultRole(index));
                }
                _ if other && !results.is_empty() => {
                    return Err(SamplingError::MixedToolResults(index));
                }
                _ => {}
            }
            results.sort_unstable();
            if results != open {
                return Err(if open.is_empty() {
                    SamplingError::UnmatchedToolResult(index)
                } else {
                    SamplingError::UnansweredToolUse(index - 1)
                });
            }
            uses.sort_unstable();
            open = uses;
        }
        open.is_empty()
            .ok_or_else(|| SamplingError::UnansweredToolUse(self.messages.len() - 1))
    }

    /// Convert to JSON, moving messages, schemas and metadata instead of copying them.
    ///
    /// # Errors
    ///
    /// Fails as `serde_json::to_value` does: on an invalid annotation priority
    /// or embedded resource.
    pub fn into_value(self) -> Result<Value, serde_json::Error> {
        let mut object = crate::utils::json_object([
            (
                "messages".into(),
                try_array(self.messages, SamplingMessage::into_value)?,
            ),
            ("maxTokens".into(), self.max_tokens.into()),
        ]);
        insert_string(&mut object, "systemPrompt", self.system_prompt);
        if let Some(temperature) = self.temperature {
            object.insert("temperature".into(), temperature.into());
        }
        if let Some(stop_sequences) = self.stop_sequences {
            object.insert(
                "stopSequences".into(),
                Value::Array(stop_sequences.into_iter().map(Value::String).collect()),
            );
        }
        if let Some(tools) = self.tools {
            object.insert("tools".into(), try_array(tools, SamplingTool::into_value)?);
        }
        insert_object(&mut object, "metadata", self.metadata);
        insert_object(&mut object, "_meta", self.meta);
        insert_serialized(&mut object, "modelPreferences", self.model_preferences)?;
        insert_serialized(&mut object, "includeContext", self.include_context)?;
        insert_serialized(&mut object, "toolChoice", self.tool_choice)?;
        insert_serialized(&mut object, "task", self.task)?;
        Ok(Value::Object(object))
    }
}

/// Convert every item into a JSON array, stopping at the first failure.
fn try_array<T>(
    items: Vec<T>,
    into_value: impl FnMut(T) -> Result<Value, serde_json::Error>,
) -> Result<Value, serde_json::Error> {
    items
        .into_iter()
        .map(into_value)
        .collect::<Result<_, _>>()
        .map(Value::Array)
}

/// Insert a small optional field that owns no payload worth moving.
fn insert_serialized(
    object: &mut Map<String, Value>,
    key: &str,
    value: Option<impl Serialize>,
) -> Result<(), serde_json::Error> {
    if let Some(value) = value {
        object.insert(key.into(), serde_json::to_value(value)?);
    }
    Ok(())
}

/// Result of creating an LLM message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMessageResult {
    /// Role of the response
    pub role: Role,
    /// Response content
    pub content: SamplingMessageContent,
    /// Model used for generation
    pub model: String,
    /// Why generation stopped
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
    /// Additional result metadata
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<Map<String, Value>>,
}

impl CreateMessageResult {
    /// Stop reason: natural end of turn
    pub const STOP_REASON_END_TURN: &'static str = "endTurn";
    /// Stop reason: hit a stop sequence
    pub const STOP_REASON_STOP_SEQUENCE: &'static str = "stopSequence";
    /// Stop reason: reached max tokens
    pub const STOP_REASON_MAX_TOKENS: &'static str = "maxTokens";
    /// Stop reason: model requested one or more tool calls
    pub const STOP_REASON_TOOL_USE: &'static str = "toolUse";
}

/// Action taken by user in response to elicitation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ElicitationAction {
    /// User approved and provided data
    Accept,
    /// User explicitly declined
    Decline,
    /// User dismissed without decision
    Cancel,
}

/// Request to elicit structured input from user
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateElicitationRequest {
    /// Message to show user
    pub message: String,
    /// Schema for requested data (imported from elicitation module)
    pub requested_schema: Value,
}

/// Result of elicitation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateElicitationResult {
    /// User's action
    pub action: ElicitationAction,
    /// User-provided content (only present if action is Accept)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Value>,
}

#[cfg(test)]
mod tests;
