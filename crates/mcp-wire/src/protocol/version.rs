//! Protocol version negotiation
//!
//! MCP supports multiple protocol versions. This module handles version negotiation
//! between client and server.

use thiserror::Error;

/// Latest supported MCP protocol version
pub const LATEST_PROTOCOL_VERSION: &str = ProtocolVersion::LATEST.as_str();

/// Macro for creating compile-time constant string types
///
/// This creates a zero-sized type that serializes/deserializes to a specific string value.
/// Useful for JSON schema type fields that must always be a specific constant.
/// Serialization emits the literal; deserialization compares borrowed input
/// and rejects any other value, without allocating on success.
#[macro_export]
macro_rules! const_string {
    ($(#[$meta:meta])* $name:ident = $value:literal) => {
        $(#[$meta])*
        #[doc = concat!("Compile-time string constant for `", $value, "`.")]
        #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
        pub struct $name;

        impl $name {
            #[doc = concat!("The string value `", $value, "`.")]
            pub const VALUE: &'static str = $value;

            /// Return the constant string value.
            pub const fn as_str(&self) -> &'static str {
                Self::VALUE
            }
        }

        impl serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_str($value)
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<$name, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct ConstVisitor;

                impl serde::de::Visitor<'_> for ConstVisitor {
                    type Value = $name;

                    fn expecting(
                        &self,
                        formatter: &mut ::std::fmt::Formatter<'_>,
                    ) -> ::std::fmt::Result {
                        formatter.write_str(concat!("the string ", stringify!($value)))
                    }

                    fn visit_str<E>(self, value: &str) -> Result<$name, E>
                    where
                        E: serde::de::Error,
                    {
                        if value == $value {
                            Ok($name)
                        } else {
                            Err(E::custom(format_args!(
                                "expected const string value {:?}, but got {:?}",
                                $value, value
                            )))
                        }
                    }
                }

                deserializer.deserialize_str(ConstVisitor)
            }
        }
    };
}

pub use const_string;

/// JSON-RPC version (always "2.0")
pub const JSON_RPC_VERSION: &str = "2.0";

/// All supported protocol versions in order of preference
///
/// Order matters: first match wins during negotiation
pub const SUPPORTED_PROTOCOL_VERSIONS: &[&str] = &[
    ProtocolVersion::ALL[0].as_str(),
    ProtocolVersion::ALL[1].as_str(),
];

/// Default minimum client versions
///
/// Can be overridden by specific implementations
pub const DEFAULT_MIN_CLIENT_VERSIONS: &[(&str, &str)] =
    &[("claude-code", "2.0.56"), ("codex-mcp-client", "0.63.0")];

/// A protocol version this server speaks; a later revision compares greater.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProtocolVersion {
    /// MCP 2025-06-18
    V2025_06_18,
    /// MCP 2025-11-25
    V2025_11_25,
}

impl ProtocolVersion {
    /// Every version, newest (preferred) first.
    pub const ALL: [Self; 2] = [Self::V2025_11_25, Self::V2025_06_18];

    /// The version the server prefers.
    pub const LATEST: Self = Self::ALL[0];

    /// The version named `version`, if this server speaks it.
    pub fn parse(version: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|known| known.as_str() == version)
    }

    /// The `protocolVersion` string.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::V2025_06_18 => "2025-06-18",
            Self::V2025_11_25 => "2025-11-25",
        }
    }
}

impl std::fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Something a session's protocol version does not define, so its client is never sent it.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{feature} is not defined in protocol version {version}")]
pub struct UnsupportedByVersion {
    /// What was refused, such as `URL-mode elicitation`.
    pub feature: String,
    /// The session's protocol version.
    pub version: ProtocolVersion,
}

/// Check if a protocol version is supported
pub fn is_supported_protocol_version(version: &str) -> bool {
    ProtocolVersion::parse(version).is_some()
}

/// Return the server's preferred protocol version.
pub const fn latest_supported_protocol_version() -> &'static str {
    ProtocolVersion::LATEST.as_str()
}

/// Negotiate protocol version between client and server
///
/// Returns the client's version if supported, otherwise the server's latest
/// supported version. Per MCP, the client should disconnect if it cannot
/// support the server-selected version.
pub fn negotiate_protocol_version(client_version: &str) -> ProtocolVersion {
    ProtocolVersion::parse(client_version).unwrap_or(ProtocolVersion::LATEST)
}

#[cfg(test)]
mod tests;
