//! Wire types of the Model Context Protocol.
//!
//! The messages, content blocks, capabilities and listing entries that an MCP
//! server and client exchange, with the protocol's own validation rules.
//! `mcp-host` (servers) and `mcp-guest` (clients) both build on this crate, so
//! the two sides share one definition of the protocol.

macro_rules! fluent_setter {
    (
        $(#[$meta:meta])*
        $vis:vis fn $method:ident($argument:ident: $argument_type:ty) => some $field:ident
    ) => {
        $(#[$meta])*
        #[must_use]
        $vis fn $method(mut self, $argument: $argument_type) -> Self {
            self.$field = Some($argument);
            self
        }
    };
    (
        $(#[$meta:meta])*
        $vis:vis fn $method:ident($argument:ident: $argument_type:ty) => some_into $field:ident
    ) => {
        $(#[$meta])*
        #[must_use]
        $vis fn $method(mut self, $argument: $argument_type) -> Self {
            self.$field = Some($argument.into());
            self
        }
    };
    (
        $(#[$meta:meta])*
        $vis:vis fn $method:ident($argument:ident: $argument_type:ty) => assign $field:ident
    ) => {
        $(#[$meta])*
        #[must_use]
        $vis fn $method(mut self, $argument: $argument_type) -> Self {
            self.$field = $argument;
            self
        }
    };
    (
        $(#[$meta:meta])*
        $vis:vis fn $method:ident($argument:ident: $argument_type:ty) => into $field:ident
    ) => {
        $(#[$meta])*
        #[must_use]
        $vis fn $method(mut self, $argument: $argument_type) -> Self {
            self.$field = $argument.into();
            self
        }
    };
}

pub mod content;
pub mod logging;
pub mod protocol;
pub mod tools;
mod utils;

#[cfg(test)]
mod test_support;
