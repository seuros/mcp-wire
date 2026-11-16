//! MCP protocol types and utilities

macro_rules! optional_field_struct {
    (
        $(#[$struct_meta:meta])*
        pub struct $name:ident {
            $(#[$field_meta:meta])*
            $field:ident: $value:ty
        }
    ) => {
        $(#[$struct_meta])*
        pub struct $name {
            $(#[$field_meta])*
            #[serde(skip_serializing_if = "Option::is_none")]
            pub $field: Option<$value>,
        }
    };
}

pub mod apps;
pub mod capabilities;
pub mod elicitation;
pub mod errors;
pub mod methods;
pub mod types;
pub mod version;
