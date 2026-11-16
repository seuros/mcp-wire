//! MCP method names
//!
//! Defines all available MCP methods as type-safe constants

use serde::{Deserialize, Serialize};
use std::fmt;

/// MCP method names
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpMethod {
    /// Initialize a new client/server session.
    Initialize,
    /// Check connectivity with the peer.
    Ping,

    /// List static resources.
    #[serde(rename = "resources/list")]
    ResourcesList,
    /// List resource templates.
    #[serde(rename = "resources/templates/list")]
    ResourcesTemplatesList,
    /// Read a resource by URI.
    #[serde(rename = "resources/read")]
    ResourcesRead,
    /// Subscribe to resource updates.
    #[serde(rename = "resources/subscribe")]
    ResourcesSubscribe,
    /// Unsubscribe from resource updates.
    #[serde(rename = "resources/unsubscribe")]
    ResourcesUnsubscribe,

    /// List prompts.
    #[serde(rename = "prompts/list")]
    PromptsList,
    /// Get a prompt by name.
    #[serde(rename = "prompts/get")]
    PromptsGet,

    /// List tools.
    #[serde(rename = "tools/list")]
    ToolsList,
    /// Call a tool by name.
    #[serde(rename = "tools/call")]
    ToolsCall,

    /// Get task metadata.
    #[serde(rename = "tasks/get")]
    TasksGet,
    /// Get task result payload.
    #[serde(rename = "tasks/result")]
    TasksResult,
    /// List tasks.
    #[serde(rename = "tasks/list")]
    TasksList,
    /// Cancel a task.
    #[serde(rename = "tasks/cancel")]
    TasksCancel,

    /// Set client logging level.
    #[serde(rename = "logging/setLevel")]
    LoggingSetLevel,

    /// Complete prompt or resource arguments.
    #[serde(rename = "completion/complete")]
    CompletionComplete,

    /// Request user input from the client.
    #[serde(rename = "elicitation/create")]
    ElicitationCreate,

    /// Request LLM sampling from the client.
    #[serde(rename = "sampling/createMessage")]
    SamplingCreateMessage,

    /// List client workspace roots.
    #[serde(rename = "roots/list")]
    RootsList,

    /// Client initialization-complete notification.
    #[serde(rename = "notifications/initialized")]
    NotificationsInitialized,
    /// Resource list changed notification.
    #[serde(rename = "notifications/resources/list_changed")]
    NotificationsResourcesListChanged,
    /// Resource updated notification.
    #[serde(rename = "notifications/resources/updated")]
    NotificationsResourcesUpdated,
    /// Prompt list changed notification.
    #[serde(rename = "notifications/prompts/list_changed")]
    NotificationsPromptsListChanged,
    /// Tool list changed notification.
    #[serde(rename = "notifications/tools/list_changed")]
    NotificationsToolsListChanged,
    /// Progress update notification.
    #[serde(rename = "notifications/progress")]
    NotificationsProgress,
    /// Logging message notification.
    #[serde(rename = "notifications/message")]
    NotificationsMessage,
    /// Request cancellation notification.
    #[serde(rename = "notifications/cancelled")]
    NotificationsCancelled,
    /// Roots list changed notification.
    #[serde(rename = "notifications/roots/list_changed")]
    NotificationsRootsListChanged,
    /// Task status changed notification.
    #[serde(rename = "notifications/tasks/status")]
    NotificationsTasksStatus,
    /// URL-mode elicitation completion notification.
    #[serde(rename = "notifications/elicitation/complete")]
    NotificationsElicitationComplete,

    /// Custom or unknown method name.
    #[serde(untagged)]
    Custom(String),
}

impl fmt::Display for McpMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Initialize => write!(f, "initialize"),
            Self::Ping => write!(f, "ping"),
            Self::ResourcesList => write!(f, "resources/list"),
            Self::ResourcesTemplatesList => write!(f, "resources/templates/list"),
            Self::ResourcesRead => write!(f, "resources/read"),
            Self::ResourcesSubscribe => write!(f, "resources/subscribe"),
            Self::ResourcesUnsubscribe => write!(f, "resources/unsubscribe"),
            Self::PromptsList => write!(f, "prompts/list"),
            Self::PromptsGet => write!(f, "prompts/get"),
            Self::ToolsList => write!(f, "tools/list"),
            Self::ToolsCall => write!(f, "tools/call"),
            Self::TasksGet => write!(f, "tasks/get"),
            Self::TasksResult => write!(f, "tasks/result"),
            Self::TasksList => write!(f, "tasks/list"),
            Self::TasksCancel => write!(f, "tasks/cancel"),
            Self::LoggingSetLevel => write!(f, "logging/setLevel"),
            Self::CompletionComplete => write!(f, "completion/complete"),
            Self::ElicitationCreate => write!(f, "elicitation/create"),
            Self::SamplingCreateMessage => write!(f, "sampling/createMessage"),
            Self::RootsList => write!(f, "roots/list"),
            Self::NotificationsInitialized => write!(f, "notifications/initialized"),
            Self::NotificationsResourcesListChanged => {
                write!(f, "notifications/resources/list_changed")
            }
            Self::NotificationsResourcesUpdated => {
                write!(f, "notifications/resources/updated")
            }
            Self::NotificationsPromptsListChanged => {
                write!(f, "notifications/prompts/list_changed")
            }
            Self::NotificationsToolsListChanged => {
                write!(f, "notifications/tools/list_changed")
            }
            Self::NotificationsProgress => write!(f, "notifications/progress"),
            Self::NotificationsMessage => write!(f, "notifications/message"),
            Self::NotificationsCancelled => write!(f, "notifications/cancelled"),
            Self::NotificationsRootsListChanged => {
                write!(f, "notifications/roots/list_changed")
            }
            Self::NotificationsTasksStatus => write!(f, "notifications/tasks/status"),
            Self::NotificationsElicitationComplete => {
                write!(f, "notifications/elicitation/complete")
            }
            Self::Custom(s) => write!(f, "{s}"),
        }
    }
}

impl From<String> for McpMethod {
    fn from(s: String) -> Self {
        Self::parse(&s).unwrap_or(Self::Custom(s))
    }
}

impl McpMethod {
    /// The standard method named `s`, without allocating; `None` for custom methods.
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "initialize" => Self::Initialize,
            "ping" => Self::Ping,
            "resources/list" => Self::ResourcesList,
            "resources/templates/list" => Self::ResourcesTemplatesList,
            "resources/read" => Self::ResourcesRead,
            "resources/subscribe" => Self::ResourcesSubscribe,
            "resources/unsubscribe" => Self::ResourcesUnsubscribe,
            "prompts/list" => Self::PromptsList,
            "prompts/get" => Self::PromptsGet,
            "tools/list" => Self::ToolsList,
            "tools/call" => Self::ToolsCall,
            "tasks/get" => Self::TasksGet,
            "tasks/result" => Self::TasksResult,
            "tasks/list" => Self::TasksList,
            "tasks/cancel" => Self::TasksCancel,
            "logging/setLevel" => Self::LoggingSetLevel,
            "completion/complete" => Self::CompletionComplete,
            "elicitation/create" => Self::ElicitationCreate,
            "sampling/createMessage" => Self::SamplingCreateMessage,
            "roots/list" => Self::RootsList,
            "notifications/initialized" => Self::NotificationsInitialized,
            "notifications/resources/list_changed" => Self::NotificationsResourcesListChanged,
            "notifications/resources/updated" => Self::NotificationsResourcesUpdated,
            "notifications/prompts/list_changed" => Self::NotificationsPromptsListChanged,
            "notifications/tools/list_changed" => Self::NotificationsToolsListChanged,
            "notifications/progress" => Self::NotificationsProgress,
            "notifications/message" => Self::NotificationsMessage,
            "notifications/cancelled" => Self::NotificationsCancelled,
            "notifications/roots/list_changed" => Self::NotificationsRootsListChanged,
            "notifications/tasks/status" => Self::NotificationsTasksStatus,
            "notifications/elicitation/complete" => Self::NotificationsElicitationComplete,
            _ => return None,
        })
    }
}

impl From<&str> for McpMethod {
    fn from(s: &str) -> Self {
        Self::parse(s).unwrap_or_else(|| Self::Custom(s.to_owned()))
    }
}

#[cfg(test)]
mod tests;
