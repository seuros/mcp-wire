use serde::{Deserialize, Serialize};

/// Task status in lifecycle
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// The request is currently being processed
    Working,
    /// The task is waiting for input (e.g., elicitation or sampling)
    InputRequired,
    /// The request completed successfully and results are available
    Completed,
    /// The associated request did not complete successfully
    Failed,
    /// The request was cancelled before completion
    Cancelled,
}

impl TaskStatus {
    /// Wire name, as produced by the `snake_case` serde rename.
    const fn as_str(self) -> &'static str {
        match self {
            Self::Working => "working",
            Self::InputRequired => "input_required",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

optional_field_struct! {
    /// Metadata for augmenting a request with task execution
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct TaskMetadata {
        /// Requested duration in milliseconds to retain task from creation
        ttl: u64
    }
}

/// Data associated with a task
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    /// The task identifier
    pub task_id: String,
    /// Current task state
    pub status: TaskStatus,
    /// Optional human-readable message describing the current task state
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_message: Option<String>,
    /// ISO 8601 timestamp when the task was created
    pub created_at: String,
    /// ISO 8601 timestamp when the task was last updated
    pub last_updated_at: String,
    /// Actual retention duration from creation in milliseconds, null for unlimited
    pub ttl: Option<u64>,
    /// Suggested polling interval in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poll_interval: Option<u64>,
}

impl Task {
    /// Convert to JSON by moving owned identifiers, timestamps and messages.
    pub fn into_value(self) -> serde_json::Value {
        use serde_json::Value;
        let mut object = crate::utils::json_object([
            ("taskId".into(), Value::String(self.task_id)),
            (
                "status".into(),
                Value::String(self.status.as_str().to_owned()),
            ),
            ("createdAt".into(), Value::String(self.created_at)),
            ("lastUpdatedAt".into(), Value::String(self.last_updated_at)),
            ("ttl".into(), self.ttl.map_or(Value::Null, Value::from)),
        ]);
        crate::content::types::insert_string(&mut object, "statusMessage", self.status_message);
        if let Some(interval) = self.poll_interval {
            object.insert("pollInterval".into(), interval.into());
        }
        Value::Object(object)
    }
}

/// A response to a task-augmented request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskResult {
    /// The created task
    pub task: Task,
}

/// Request params for tasks/get
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTaskParams {
    /// The task identifier to query
    pub task_id: String,
}

/// Request params for tasks/cancel
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelTaskParams {
    /// The task identifier to cancel
    pub task_id: String,
}

optional_field_struct! {
    /// Tool execution metadata
    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ToolExecution {
        /// Indicates whether this tool supports task-augmented execution
        task_support: TaskSupport
    }
}

/// Task support level for a tool
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TaskSupport {
    /// Tool does not support task-augmented execution (default when absent)
    Forbidden,
    /// Tool may support task-augmented execution
    Optional,
    /// Tool requires task-augmented execution
    Required,
}

/// Tool annotations (hints for LLMs about tool behavior)
///
/// NOTE: All properties are **hints**. They are not guaranteed to provide
/// a faithful description of tool behavior. Clients should never make
/// tool use decisions based on annotations from untrusted servers.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ToolAnnotations {
    /// Human-readable title for the tool
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// If true, the tool does not modify its environment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_only_hint: Option<bool>,
    /// If true, the tool may perform destructive updates to its environment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destructive_hint: Option<bool>,
    /// If true, calling the tool repeatedly with the same arguments
    /// will have no additional effect on its environment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotent_hint: Option<bool>,
    /// If true, this tool may interact with an "open world" of external entities.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_world_hint: Option<bool>,
}

impl ToolAnnotations {
    /// Convert to JSON, moving an owned annotation title.
    pub fn into_value(self) -> serde_json::Value {
        use serde_json::{Map, Value};
        let mut object = Map::new();
        crate::content::types::insert_string(&mut object, "title", self.title);
        for (key, value) in [
            ("readOnlyHint", self.read_only_hint),
            ("destructiveHint", self.destructive_hint),
            ("idempotentHint", self.idempotent_hint),
            ("openWorldHint", self.open_world_hint),
        ] {
            if let Some(value) = value {
                object.insert(key.into(), Value::Bool(value));
            }
        }
        Value::Object(object)
    }
}
