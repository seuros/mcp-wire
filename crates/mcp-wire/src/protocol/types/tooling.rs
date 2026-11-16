use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub use crate::content::types::ContentBlock as ContentItem;
use crate::content::types::insert_object;

use super::{ProgressToken, RequestId, TaskMetadata};

/// Request to set the minimum logging level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetLevelRequest {
    /// Minimum log level to receive
    pub level: crate::logging::LogLevel,
}

/// Parameters for a `tools/call` request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallToolRequestParams {
    /// Name of the tool to call
    pub name: String,
    /// Tool arguments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<serde_json::Map<String, Value>>,
    /// Task augmentation requested for this tool call
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<TaskMetadata>,
    /// Request metadata
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<Map<String, Value>>,
}

impl CallToolRequestParams {
    /// Decode owned params, moving `arguments` and `_meta` where
    /// `serde_json::from_value` would rebuild every map and array in them.
    ///
    /// # Errors
    ///
    /// Fails as `serde_json::from_value` does, with the same messages.
    pub fn from_value(value: Value) -> Result<Self, serde_json::Error> {
        let Value::Object(mut object) = value else {
            return serde_json::from_value(value);
        };
        // Fields in key order, so the first error matches serde's.
        let meta = take_object(&mut object, "_meta")?;
        let arguments = take_object(&mut object, "arguments")?;
        let name = object.remove("name").map(String::deserialize).transpose()?;
        let task = object
            .remove("task")
            .map_or(Ok(None), Option::deserialize)?;
        Ok(Self {
            name: name.ok_or_else(|| serde::de::Error::missing_field("name"))?,
            arguments,
            task,
            meta,
        })
    }
}

/// Move an optional object field out of `object`; null reads as absent.
fn take_object(
    object: &mut Map<String, Value>,
    key: &str,
) -> Result<Option<Map<String, Value>>, serde_json::Error> {
    match object.remove(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Object(map)) => Ok(Some(map)),
        // Not an object: let serde report it.
        Some(other) => Map::deserialize(other).map(Some),
    }
}

/// Result of a `tools/call` request
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CallToolResult {
    /// Content items returned by the tool
    pub content: Vec<ContentItem>,
    /// Whether this result represents an error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
    /// Structured content for machine consumption
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_content: Option<Map<String, Value>>,
    /// Response metadata
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<Map<String, Value>>,
}

impl CallToolResult {
    /// Convert to JSON, consuming content and structured payload buffers.
    /// Panics if content violates its serialization invariants.
    pub fn into_value(self) -> Value {
        let mut object = Map::new();
        object.insert(
            "content".into(),
            Value::Array(
                self.content
                    .into_iter()
                    .map(ContentItem::into_value)
                    .collect(),
            ),
        );
        if let Some(is_error) = self.is_error {
            object.insert("isError".into(), Value::Bool(is_error));
        }
        insert_object(&mut object, "structuredContent", self.structured_content);
        insert_object(&mut object, "_meta", self.meta);
        Value::Object(object)
    }

    /// Create a successful text result
    pub fn text(s: impl Into<String>) -> Self {
        Self {
            content: vec![ContentItem::text(s)],
            is_error: None,
            structured_content: None,
            meta: None,
        }
    }

    /// Create an error text result
    pub fn error(s: impl Into<String>) -> Self {
        Self {
            content: vec![ContentItem::text(s)],
            is_error: Some(true),
            structured_content: None,
            meta: None,
        }
    }
}

/// Result of a `tools/list` request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListToolsResult {
    /// Available tools
    pub tools: Vec<Value>,
    /// Cursor for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    /// Response metadata
    #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
    pub meta: Option<Map<String, Value>>,
}

/// Parameters for paginated requests
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PaginatedRequestParams {
    /// Cursor for the next page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// Parameters for a `notifications/cancelled` notification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelledNotificationParams {
    /// The ID of the request that was cancelled
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<RequestId>,
    /// Optional reason for cancellation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Parameters for a `notifications/progress` notification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressNotificationParams {
    /// Progress token from the original request
    pub progress_token: ProgressToken,
    /// Progress value (0.0 - 1.0)
    pub progress: f64,
    /// Total expected value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<f64>,
    /// Optional message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[cfg(test)]
mod tests;
