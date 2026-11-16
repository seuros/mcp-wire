//! The `tools/list` entry and the protocol's rules for tool names and schemas.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use thiserror::Error;

use crate::content::types::Icon;

/// A tool definition that breaks the protocol's rules for names and schemas.
#[derive(Debug, Error)]
pub enum ToolDefinitionError {
    /// The name breaks the MCP tool name rules; see [`is_valid_tool_name`].
    #[error("tool name '{tool}' must be 1-128 characters of A-Z, a-z, 0-9, '_', '-' or '.'")]
    InvalidName {
        /// Rejected tool name or alias.
        tool: String,
    },
    /// The tool's name differs from the name it is registered under.
    #[error("tool '{tool}' cannot be registered under the name '{key}'")]
    NameMismatch {
        /// Name the tool was registered under.
        key: String,
        /// The tool's own name.
        tool: String,
    },
    /// The input schema is not an object-root JSON Schema.
    #[error("tool '{tool}' input schema must be a JSON object with type \"object\"")]
    InvalidInputSchema {
        /// Tool whose schema was rejected.
        tool: String,
    },
    /// The output schema is not an object-root JSON Schema.
    #[error("tool '{tool}' output schema must be a JSON object with type \"object\"")]
    InvalidOutputSchema {
        /// Tool whose schema was rejected.
        tool: String,
    },
}

/// Whether `name` is 1-128 ASCII letters, digits, `_`, `-` or `.`. `const` so macros can check it at compile time.
pub const fn is_valid_tool_name(name: &str) -> bool {
    let mut rest = name.as_bytes();
    if rest.is_empty() || rest.len() > 128 {
        return false;
    }
    while let [byte, tail @ ..] = rest {
        if !(byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.')) {
            return false;
        }
        rest = tail;
    }
    true
}

/// Whether `schema` is a JSON object with `"type": "object"`.
fn has_object_root(schema: &Value) -> bool {
    schema.get("type").and_then(Value::as_str) == Some("object")
}

/// Check a tool definition's schemas have object roots (schema.ts `Tool`).
///
/// # Errors
///
/// The first schema without an object root, input before output.
pub fn validate_tool_schemas(
    name: &str,
    input_schema: &Value,
    output_schema: Option<&Value>,
) -> Result<(), ToolDefinitionError> {
    let tool = || name.to_owned();
    has_object_root(input_schema)
        .ok_or_else(|| ToolDefinitionError::InvalidInputSchema { tool: tool() })?;
    output_schema
        .is_none_or(has_object_root)
        .ok_or_else(|| ToolDefinitionError::InvalidOutputSchema { tool: tool() })
}

/// Tool metadata for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    /// Tool name (programmatic identifier)
    pub name: String,

    /// Human-readable title for UI display
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Tool description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Sized icons for client display
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::content::types::received_icons"
    )]
    pub icons: Option<Vec<Icon>>,

    /// Input schema (JSON Schema)
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,

    /// Output schema (JSON Schema) for structured output
    #[serde(rename = "outputSchema", skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<Value>,

    /// Execution metadata (task support, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution: Option<crate::protocol::types::ToolExecution>,

    /// Tool annotations (behavioral hints for LLMs)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<crate::protocol::types::ToolAnnotations>,

    /// Additional tool metadata
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<Map<String, Value>>,
}

impl ToolInfo {
    /// Check that the name is `key` and valid, and both schemas have an object root.
    ///
    /// # Errors
    ///
    /// The first rule the definition breaks: name match, name, then schemas.
    pub fn validate(&self, key: &str) -> Result<(), ToolDefinitionError> {
        let tool = || self.name.clone();
        (self.name == key).ok_or_else(|| ToolDefinitionError::NameMismatch {
            key: key.to_owned(),
            tool: tool(),
        })?;
        is_valid_tool_name(key).ok_or_else(|| ToolDefinitionError::InvalidName { tool: tool() })?;
        validate_tool_schemas(&self.name, &self.input_schema, self.output_schema.as_ref())
    }

    /// Convert owned listing metadata to JSON without copying schemas or strings.
    ///
    /// # Errors
    ///
    /// Returns a [`serde_json::Error`] when `execution` fails to serialize.
    pub fn into_value(self) -> Result<Value, serde_json::Error> {
        let mut object = listing_object(
            self.name,
            self.title,
            self.description,
            self.icons,
            self.meta,
        );
        object.insert("inputSchema".into(), self.input_schema);
        if let Some(schema) = self.output_schema {
            object.insert("outputSchema".into(), schema);
        }
        if let Some(execution) = self.execution {
            object.insert("execution".into(), serde_json::to_value(execution)?);
        }
        if let Some(annotations) = self.annotations {
            object.insert("annotations".into(), annotations.into_value());
        }
        Ok(Value::Object(object))
    }
}

/// The fields every listing entry shares (`name`, `title`, `description`, `icons`, `_meta`).
pub fn listing_object(
    name: String,
    title: Option<String>,
    description: Option<String>,
    icons: Option<Vec<crate::content::types::Icon>>,
    meta: Option<Map<String, Value>>,
) -> Map<String, Value> {
    let mut object = Map::new();
    object.insert("name".into(), Value::String(name));
    crate::content::types::insert_string(&mut object, "title", title);
    crate::content::types::insert_string(&mut object, "description", description);
    if let Some(icons) = icons {
        object.insert(
            "icons".into(),
            Value::Array(
                icons
                    .into_iter()
                    .map(crate::content::types::Icon::into_value)
                    .collect(),
            ),
        );
    }
    crate::content::types::insert_object(&mut object, "_meta", meta);
    object
}
