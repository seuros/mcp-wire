//! MCP capability negotiation types
//!
//! Capabilities define what features both client and server support.
//! These are exchanged during the initialize handshake.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use super::apps;
use super::types::Implementation;

/// Server capabilities
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerCapabilities {
    /// Logging capability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<LoggingCapability>,

    /// Tools capability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsCapability>,

    /// Resources capability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourcesCapability>,

    /// Prompts capability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptsCapability>,

    /// Completions capability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completions: Option<CompletionCapability>,

    /// Tasks capability (supports async task execution)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tasks: Option<TasksCapability>,

    /// Experimental/custom capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<HashMap<String, Value>>,

    /// Supported MCP extensions, keyed by extension identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<HashMap<String, Value>>,
}

impl ServerCapabilities {
    /// Whether the server supports task-augmented `tools/call`.
    pub fn supports_task_tool_calls(&self) -> bool {
        self.tasks
            .as_ref()
            .and_then(|tasks| tasks.requests.as_ref())
            .and_then(|requests| requests.tools.as_ref())
            .and_then(|tools| tools.call.as_ref())
            .is_some()
    }
}

/// Client capabilities
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientCapabilities {
    /// Roots capability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roots: Option<RootsCapability>,

    /// Sampling capability (client can perform LLM sampling)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<SamplingCapability>,

    /// Elicitation capability (client can request user input)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elicitation: Option<ElicitationCapability>,

    /// Tasks capability (supports async task execution)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tasks: Option<TasksCapability>,

    /// Experimental/custom capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<HashMap<String, Value>>,

    /// Supported MCP extensions, keyed by extension identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<HashMap<String, Value>>,
}

/// Logging capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingCapability {}

optional_field_struct! {
    /// Tools capability
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ToolsCapability {
        /// Whether tools support list changes notification
        list_changed: bool
    }
}

/// Resources capability
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourcesCapability {
    /// Whether resources support subscription
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscribe: Option<bool>,

    /// Whether resources support list changes notification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

optional_field_struct! {
    /// Prompts capability
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct PromptsCapability {
        /// Whether prompts support list changes notification
        list_changed: bool
    }
}

/// Completion capability (autocomplete)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionCapability {}

optional_field_struct! {
    /// Roots capability (client-provided workspace roots)
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct RootsCapability {
        /// Whether roots support list changes notification
        list_changed: bool
    }
}

/// Sampling capability
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SamplingCapability {
    /// Whether the client supports context inclusion via includeContext.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<EmptyObject>,

    /// Whether the client supports tool use via tools and toolChoice.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<EmptyObject>,
}

impl SamplingCapability {
    /// Whether the client supports `includeContext` values beyond `none`.
    pub const fn supports_context(&self) -> bool {
        self.context.is_some()
    }

    /// Whether the client supports sampling tool use.
    pub const fn supports_tools(&self) -> bool {
        self.tools.is_some()
    }
}

/// Elicitation capability
///
/// Per MCP 2025-11-25, elicitation supports two modes:
/// - `form`: Structured schema-based input collection
/// - `url`: Opens a URL for out-of-band auth/input
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ElicitationCapability {
    /// Form-mode elicitation support
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form: Option<FormElicitationCapability>,
    /// URL-mode elicitation support
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<UrlElicitationCapability>,
}

/// Form-mode elicitation capability marker
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FormElicitationCapability {}

/// URL-mode elicitation capability marker
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UrlElicitationCapability {}

impl ElicitationCapability {
    /// Whether the client supports schema-based `elicitation/create` requests.
    ///
    /// An empty capability object declares form mode only
    /// (client/elicitation.mdx:65).
    pub const fn supports_form(&self) -> bool {
        self.form.is_some() || (self.form.is_none() && self.url.is_none())
    }

    /// Whether the client supports URL-based elicitation flows.
    pub const fn supports_url(&self) -> bool {
        self.url.is_some()
    }
}

impl ClientCapabilities {
    /// Whether the client supports `sampling/createMessage`.
    pub const fn supports_sampling(&self) -> bool {
        self.sampling.is_some()
    }

    /// Whether the client supports sampling context inclusion.
    pub fn supports_sampling_context(&self) -> bool {
        self.sampling
            .as_ref()
            .is_some_and(SamplingCapability::supports_context)
    }

    /// Whether the client supports sampling tool use.
    pub fn supports_sampling_tools(&self) -> bool {
        self.sampling
            .as_ref()
            .is_some_and(SamplingCapability::supports_tools)
    }

    /// Whether the client supports task-augmented `sampling/createMessage`.
    pub fn supports_task_sampling(&self) -> bool {
        self.tasks
            .as_ref()
            .and_then(|tasks| tasks.requests.as_ref())
            .and_then(|requests| requests.sampling.as_ref())
            .and_then(|sampling| sampling.create_message.as_ref())
            .is_some()
    }

    /// Whether the client supports schema-based `elicitation/create` requests.
    pub fn supports_form_elicitation(&self) -> bool {
        self.elicitation
            .as_ref()
            .is_some_and(ElicitationCapability::supports_form)
    }

    /// Whether the client supports URL-mode `elicitation/create` requests.
    pub fn supports_url_elicitation(&self) -> bool {
        self.elicitation
            .as_ref()
            .is_some_and(ElicitationCapability::supports_url)
    }

    /// Settings object the client advertised for an extension.
    pub fn extension(&self, id: &str) -> Option<&Value> {
        self.extensions.as_ref()?.get(id)
    }

    /// Whether the client renders MCP Apps (`text/html;profile=mcp-app` views).
    pub fn supports_mcp_apps(&self) -> bool {
        self.extension(apps::EXTENSION_ID)
            .and_then(|settings| settings.get("mimeTypes"))
            .and_then(Value::as_array)
            .is_some_and(|mime_types| {
                mime_types
                    .iter()
                    .any(|mime_type| mime_type == apps::MIME_TYPE)
            })
    }
}

/// Tasks capability
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TasksCapability {
    /// Whether this party supports tasks/list
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list: Option<EmptyObject>,

    /// Whether this party supports tasks/cancel
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancel: Option<EmptyObject>,

    /// Specifies which request types can be augmented with tasks
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requests: Option<TasksRequestsCapability>,
}

impl TasksCapability {
    /// Task-augmented `tools/call`, with `tasks/list` and `tasks/cancel` as asked.
    pub fn for_tool_calls(list: bool, cancel: bool) -> Self {
        Self {
            list: list.then_some(EmptyObject {}),
            cancel: cancel.then_some(EmptyObject {}),
            requests: Some(TasksRequestsCapability {
                tools: Some(TasksToolsCapability {
                    call: Some(EmptyObject {}),
                }),
                ..Default::default()
            }),
        }
    }
}

/// Requests that can be task-augmented
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TasksRequestsCapability {
    /// Task support for tool-related requests (server-side)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<TasksToolsCapability>,

    /// Task support for sampling-related requests (client-side)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<TasksSamplingCapability>,

    /// Task support for elicitation-related requests (client-side)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elicitation: Option<TasksElicitationCapability>,
}

optional_field_struct! {
    /// Task support for tool requests
    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct TasksToolsCapability {
        /// Whether tools/call supports task-augmented execution
        call: EmptyObject
    }
}

optional_field_struct! {
    /// Task support for sampling requests
    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct TasksSamplingCapability {
        /// Whether sampling/createMessage supports task-augmented execution
        create_message: EmptyObject
    }
}

optional_field_struct! {
    /// Task support for elicitation requests
    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct TasksElicitationCapability {
        /// Whether elicitation/create supports task-augmented execution
        create: EmptyObject
    }
}

/// Empty object placeholder (used for boolean-like capabilities)
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmptyObject {}

/// Initialize request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeRequest {
    /// Protocol version
    pub protocol_version: String,

    /// Client capabilities
    pub capabilities: ClientCapabilities,

    /// Client implementation info
    pub client_info: Implementation,
}

/// Initialize result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    /// Protocol version (negotiated)
    pub protocol_version: String,

    /// Server capabilities
    pub capabilities: ServerCapabilities,

    /// Server implementation info
    pub server_info: Implementation,

    /// Optional server instructions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
}

#[cfg(test)]
mod tests;
