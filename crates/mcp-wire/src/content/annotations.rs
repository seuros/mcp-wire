//! Content annotations
//!
//! Annotations provide metadata about content (audience, priority, timestamps, etc.)
//! Based on MCP 2025-11-25 spec and Go mcphost implementation

use serde::{Deserialize, Deserializer, Serialize};

use crate::protocol::types::Role;

/// A priority in `0.0..=1.0` (schema.ts `@minimum 0 @maximum 1`).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "f64")]
pub struct Priority(f64);

/// A priority outside `0.0..=1.0`, or NaN.
#[derive(Debug, Clone, Copy, PartialEq, thiserror::Error)]
#[error("priority must be between 0 and 1, got {0}")]
pub struct InvalidPriority(pub f64);

impl Priority {
    /// The value, within `0.0..=1.0`.
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl TryFrom<f64> for Priority {
    type Error = InvalidPriority;

    fn try_from(value: f64) -> Result<Self, InvalidPriority> {
        (0.0..=1.0)
            .contains(&value)
            .then_some(Self(value))
            .ok_or(InvalidPriority(value))
    }
}

/// Content annotations
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Annotations {
    /// Target audience for this content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audience: Option<Vec<Role>>,

    /// Priority level (0.0 to 1.0, higher = more important)
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_priority"
    )]
    pub priority: Option<Priority>,

    /// Timestamp when content was last modified (ISO 8601)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<String>,
}

/// A `priority` key holds a number; `null` is not one.
fn present_priority<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Priority>, D::Error> {
    Priority::deserialize(deserializer).map(Some)
}

impl Annotations {
    /// Convert to JSON, moving timestamps.
    pub fn into_value(self) -> serde_json::Value {
        use serde_json::{Map, Value};
        let mut object = Map::new();
        if let Some(priority) = self.priority {
            object.insert("priority".into(), Value::from(priority.get()));
        }
        if let Some(audience) = self.audience {
            object.insert(
                "audience".into(),
                Value::Array(
                    audience
                        .into_iter()
                        .map(|role| Value::String(role.as_str().to_owned()))
                        .collect(),
                ),
            );
        }
        super::types::insert_string(&mut object, "lastModified", self.last_modified);
        Value::Object(object)
    }

    /// Create new empty annotations
    pub fn new() -> Self {
        Self::default()
    }

    fluent_setter!(
        /// Set audience
        pub fn with_audience(audience: Vec<Role>) => some audience
    );

    /// Add single audience member
    #[must_use]
    pub fn add_audience(mut self, audience: Role) -> Self {
        self.audience.get_or_insert_with(Vec::new).push(audience);
        self
    }

    fluent_setter!(
        /// Set priority
        pub fn with_priority(priority: Priority) => some priority
    );

    fluent_setter!(
        /// Set last modified timestamp
        pub fn with_last_modified(timestamp: impl Into<String>) => some_into last_modified
    );
}

#[cfg(test)]
mod tests;
