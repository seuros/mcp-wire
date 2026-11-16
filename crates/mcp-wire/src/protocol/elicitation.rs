//! Type-safe schema definitions for MCP elicitation requests.
//!
//! Copied from rust-sdk and adapted for mcphost-rs architecture.
//! This module provides strongly-typed schema definitions for elicitation requests
//! that comply with the MCP 2025-11-25 specification. Elicitation schemas must be
//! objects with primitive-typed properties.
//!
//! # Example
//!
//! ```rust
//! use mcp_wire::protocol::elicitation::*;
//!
//! let schema = ElicitationSchema::builder()
//!     .required_email("email")
//!     .required_integer("age", 0, 150)?
//!     .optional_bool("newsletter", false)
//!     .build()?;
//! assert_eq!(schema.properties.len(), 3);
//! # Ok::<(), &'static str>(())
//! ```

mod decode;
mod enums;
mod primitives;
mod schema;
mod validation;

pub use enums::*;
pub use primitives::*;
pub use schema::*;
pub use validation::*;

#[cfg(test)]
mod tests;
