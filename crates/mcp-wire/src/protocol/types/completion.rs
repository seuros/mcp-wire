use std::collections::HashMap;

use serde::de::Error as _;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Maximum number of completion values allowed by the MCP specification.
pub const MAX_COMPLETION_VALUES: usize = 100;

/// Reference to what is being completed
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum CompletionRef {
    /// Completing a prompt argument
    #[serde(rename = "ref/prompt")]
    Prompt {
        /// Name of the prompt
        name: String,
        /// Optional human-readable prompt title.
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
    },
    /// Completing a resource URI
    #[serde(rename = "ref/resource")]
    Resource {
        /// URI of the resource
        uri: String,
    },
}

/// Argument being completed
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionArgument {
    /// Name of the argument being completed
    pub name: String,
    /// Current value of the argument (partial input)
    pub value: String,
}

/// Previously resolved arguments that can inform completion suggestions.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CompletionContext {
    /// Previously resolved prompt or URI-template arguments.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<HashMap<String, String>>,
}

/// Request for completion suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteRequest {
    /// Reference to what is being completed
    #[serde(rename = "ref")]
    pub reference: CompletionRef,
    /// The argument being completed
    pub argument: CompletionArgument,
    /// Optional previously resolved arguments.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<CompletionContext>,
    /// Additional request metadata.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<Map<String, Value>>,
}

/// A single string completion suggestion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CompletionValue(pub String);

impl CompletionValue {
    /// Create a simple completion value
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Borrow the completion string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for CompletionValue {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for CompletionValue {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

/// Completion result containing suggestions
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteResult {
    /// Additional result metadata.
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<Map<String, Value>>,
    /// Completion suggestions
    pub completion: CompletionInfo,
}

impl CompleteResult {
    /// Convert to JSON, transferring suggestion and metadata buffers.
    pub fn into_value(self) -> Value {
        let mut object = Map::new();
        object.insert("completion".into(), self.completion.into_value());
        crate::content::types::insert_object(&mut object, "_meta", self.meta);
        Value::Object(object)
    }
}

/// Information about completion suggestions
#[derive(Debug, Clone, Default)]
pub struct CompletionInfo {
    /// List of completion values
    pub values: Vec<CompletionValue>,
    /// Total number of completions available; never fewer than `values` on the wire
    pub total: Option<usize>,
    /// Whether there are more completions available
    pub has_more: Option<bool>,
}

impl Serialize for CompletionInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let values = &self.values[..self.values.len().min(MAX_COMPLETION_VALUES)];
        let truncated = values.len() < self.values.len();
        let has_more = if truncated { Some(true) } else { self.has_more };
        let total = self.wire_total();
        let field_count = 1 + usize::from(total.is_some()) + usize::from(has_more.is_some());
        let mut state = serializer.serialize_struct("CompletionInfo", field_count)?;
        state.serialize_field("values", values)?;
        if let Some(total) = total {
            state.serialize_field("total", &total)?;
        }
        if let Some(has_more) = has_more {
            state.serialize_field("hasMore", &has_more)?;
        }
        state.end()
    }
}

impl<'de> Deserialize<'de> for CompletionInfo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct WireCompletionInfo {
            values: Vec<CompletionValue>,
            total: Option<usize>,
            has_more: Option<bool>,
        }

        let wire = WireCompletionInfo::deserialize(deserializer)?;
        if wire.values.len() > MAX_COMPLETION_VALUES {
            return Err(D::Error::custom(format!(
                "completion.values must not contain more than {MAX_COMPLETION_VALUES} items"
            )));
        }

        Ok(Self {
            values: wire.values,
            total: wire.total,
            has_more: wire.has_more,
        })
    }
}

impl CompletionInfo {
    /// Convert to JSON with the same value limit and hasMore behavior as Serialize.
    pub fn into_value(mut self) -> Value {
        let truncated = self.values.len() > MAX_COMPLETION_VALUES;
        let total = self.wire_total();
        self.values.truncate(MAX_COMPLETION_VALUES);
        let mut object = Map::new();
        object.insert(
            "values".into(),
            Value::Array(
                self.values
                    .into_iter()
                    .map(|value| Value::String(value.0))
                    .collect(),
            ),
        );
        if let Some(total) = total {
            object.insert("total".into(), total.into());
        }
        if let Some(has_more) = if truncated { Some(true) } else { self.has_more } {
            object.insert("hasMore".into(), Value::Bool(has_more));
        }
        Value::Object(object)
    }

    /// Create empty completion info
    pub const fn empty() -> Self {
        Self {
            values: vec![],
            total: None,
            has_more: None,
        }
    }

    /// Create completion info with values
    pub fn with_values(mut values: Vec<CompletionValue>) -> Self {
        let total = values.len();
        values.truncate(MAX_COMPLETION_VALUES);
        Self {
            values,
            total: (total > MAX_COMPLETION_VALUES).then_some(total),
            has_more: (total > MAX_COMPLETION_VALUES).then_some(true),
        }
    }

    /// The total available: at least every value held, which the schema says `total` may exceed.
    fn wire_total(&self) -> Option<usize> {
        self.total.map(|total| total.max(self.values.len()))
    }

    /// Create completion info with pagination metadata; `total` is raised to `values.len()`.
    pub fn with_pagination(mut values: Vec<CompletionValue>, total: usize, has_more: bool) -> Self {
        let truncated = values.len() > MAX_COMPLETION_VALUES;
        let total = total.max(values.len());
        values.truncate(MAX_COMPLETION_VALUES);
        Self {
            values,
            total: Some(total),
            has_more: Some(has_more || truncated),
        }
    }
}
