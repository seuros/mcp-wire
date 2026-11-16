//! The protocol's log levels.

use std::str::FromStr;

/// Log level for MCP notifications (RFC 5424 compatible)
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// System is unusable (0)
    Emergency,
    /// Action must be taken immediately (1)
    Alert,
    /// Critical conditions (2)
    Critical,
    /// Error conditions (3)
    Error,
    /// Warning conditions (4)
    Warning,
    /// Normal but significant events (5)
    Notice,
    /// Informational messages (6)
    Info,
    /// Detailed debugging information (7)
    Debug,
}

impl LogLevel {
    /// Get the string representation for JSON-RPC
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Emergency => "emergency",
            Self::Alert => "alert",
            Self::Critical => "critical",
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Notice => "notice",
            Self::Info => "info",
            Self::Debug => "debug",
        }
    }
}

impl FromStr for LogLevel {
    type Err = String;

    /// Parse a wire name; the schema spells every level in lowercase.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "emergency" => Ok(Self::Emergency),
            "alert" => Ok(Self::Alert),
            "critical" => Ok(Self::Critical),
            "error" => Ok(Self::Error),
            "warning" => Ok(Self::Warning),
            "notice" => Ok(Self::Notice),
            "info" => Ok(Self::Info),
            "debug" => Ok(Self::Debug),
            _ => Err(format!("Invalid log level: {s}")),
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
