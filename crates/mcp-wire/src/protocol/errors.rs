//! MCP error types with builder pattern
//!
//! Provides rich error types inspired by the Go mcphost implementation

use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use strum::Display;
use thiserror::Error;

use super::types::{ErrorCode, JsonRpcError};

macro_rules! mcp_error_constructors {
    ($(#[$meta:meta])* $name:ident => $error_type:ident; $($rest:tt)*) => {
        $(#[$meta])*
        pub fn $name(code: impl Into<String>, message: impl Into<String>) -> Self {
            Self::builder(ErrorType::$error_type, code)
                .message(message)
                .build()
        }

        mcp_error_constructors!($($rest)*);
    };
    () => {};
}

/// Error type categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
#[strum(serialize_all = "snake_case")]
pub enum ErrorType {
    /// Validation error
    Validation,
    /// Resource not found
    NotFound,
    /// Method not found
    MethodNotFound,
    /// Unauthorized access
    Unauthorized,
    /// Internal server error
    Internal,
    /// Request timeout
    Timeout,
    /// Request cancelled
    Cancelled,
    /// Rate limit exceeded
    RateLimit,
    /// Resource conflict
    Conflict,
    /// Circuit breaker open
    CircuitOpen,
}

/// Rich MCP error with context
#[derive(Debug, Error)]
pub struct McpError {
    /// Error type
    pub error_type: ErrorType,
    /// Error code (string identifier)
    pub code: String,
    /// Error message
    pub message: String,
    /// Additional details
    pub details: HashMap<String, Value>,
    /// Underlying cause
    #[source]
    pub cause: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl fmt::Display for McpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}: {}", self.error_type, self.code, self.message)
    }
}

impl McpError {
    /// Convert to JSON-RPC error
    pub fn to_jsonrpc(&self) -> JsonRpcError {
        let mut data: serde_json::Map<String, Value> = self
            .details
            .iter()
            .filter(|(key, _)| !matches!(key.as_str(), "type" | "code"))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        data.insert("type".into(), Value::String(self.error_type.to_string()));
        data.insert("code".into(), Value::String(self.code.clone()));

        JsonRpcError::new(self.jsonrpc_code(), &self.message).with_data(Value::Object(data))
    }

    /// Consume this error, moving its message, code and detail payloads into
    /// a JSON-RPC error instead of making independently owned copies.
    pub fn into_jsonrpc(self) -> JsonRpcError {
        let code = self.jsonrpc_code();
        let mut data: serde_json::Map<String, Value> = self
            .details
            .into_iter()
            .filter(|(key, _)| !matches!(key.as_str(), "type" | "code"))
            .collect();
        data.insert("type".into(), Value::String(self.error_type.to_string()));
        data.insert("code".into(), Value::String(self.code));
        JsonRpcError::new(code, self.message).with_data(Value::Object(data))
    }

    const fn jsonrpc_code(&self) -> i32 {
        match self.error_type {
            ErrorType::Validation => ErrorCode::INVALID_PARAMS,
            ErrorType::NotFound => ErrorCode::RESOURCE_NOT_FOUND,
            ErrorType::MethodNotFound => ErrorCode::METHOD_NOT_FOUND,
            ErrorType::Unauthorized => ErrorCode::UNAUTHORIZED,
            ErrorType::Internal => ErrorCode::INTERNAL_ERROR,
            ErrorType::Timeout
            | ErrorType::Cancelled
            | ErrorType::RateLimit
            | ErrorType::Conflict
            | ErrorType::CircuitOpen => ErrorCode::SERVER_ERROR,
        }
    }

    /// Start building a new error
    pub fn builder(error_type: ErrorType, code: impl Into<String>) -> McpErrorBuilder {
        McpErrorBuilder {
            error_type,
            code: code.into(),
            message: String::new(),
            details: HashMap::new(),
            cause: None,
        }
    }

    mcp_error_constructors! {
        /// Create a validation error
        validation => Validation;
        /// Create a not found error
        not_found => NotFound;
        /// Create an internal error
        internal => Internal;
    }

    /// Create a method not found error
    pub fn method_not_found(method: impl Into<String>) -> Self {
        let method = method.into();
        Self::builder(ErrorType::MethodNotFound, "method_not_found")
            .message(format!("Method not found: {method}"))
            .detail("method", method)
            .build()
    }

    /// Create a not implemented error
    pub fn not_implemented(message: impl Into<String>) -> Self {
        Self::builder(ErrorType::Internal, "not_implemented")
            .message(message)
            .build()
    }

    /// Create a rate limited error
    pub fn rate_limited(message: impl Into<String>) -> Self {
        Self::builder(ErrorType::RateLimit, "rate_limit_exceeded")
            .message(message)
            .build()
    }

    /// Create a circuit breaker open error
    pub fn circuit_open(tool_name: impl Into<String>, message: impl Into<String>) -> Self {
        let tool = tool_name.into();
        Self::builder(ErrorType::CircuitOpen, "circuit_breaker_open")
            .message(message)
            .detail("tool", tool)
            .build()
    }
}

/// Builder for `McpError`
pub struct McpErrorBuilder {
    error_type: ErrorType,
    code: String,
    message: String,
    details: HashMap<String, Value>,
    cause: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl McpErrorBuilder {
    /// Set error message
    #[must_use]
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }

    /// Add a detail field
    #[must_use]
    pub fn detail(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.details.insert(key.into(), value.into());
        self
    }

    /// Set the underlying cause
    #[must_use]
    pub fn cause(mut self, cause: impl std::error::Error + Send + Sync + 'static) -> Self {
        self.cause = Some(Box::new(cause));
        self
    }

    /// Build the error
    pub fn build(self) -> McpError {
        McpError {
            error_type: self.error_type,
            code: self.code,
            message: self.message,
            details: self.details,
            cause: self.cause,
        }
    }
}

/// MCP-specific error codes
pub mod codes {
    /// Resource not found
    pub const RESOURCE_NOT_FOUND: &str = "RESOURCE_NOT_FOUND";
    /// Tool not found
    pub const TOOL_NOT_FOUND: &str = "TOOL_NOT_FOUND";
    /// Prompt not found
    pub const PROMPT_NOT_FOUND: &str = "PROMPT_NOT_FOUND";
    /// Invalid tool arguments
    pub const INVALID_TOOL_ARGS: &str = "INVALID_TOOL_ARGS";
    /// Tool output that breaks the tool's output schema
    pub const INVALID_TOOL_OUTPUT: &str = "INVALID_TOOL_OUTPUT";
    /// Tool execution failed
    pub const TOOL_EXECUTION_FAILED: &str = "TOOL_EXECUTION_FAILED";
    /// Circuit breaker open
    pub const CIRCUIT_BREAKER_OPEN: &str = "CIRCUIT_BREAKER_OPEN";
    /// Rate limit exceeded
    pub const RATE_LIMIT_EXCEEDED: &str = "RATE_LIMIT_EXCEEDED";
    /// Tool call exceeded its configured timeout
    pub const TOOL_TIMEOUT: &str = "TOOL_TIMEOUT";
}

#[cfg(test)]
mod tests;
