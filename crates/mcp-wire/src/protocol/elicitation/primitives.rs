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

use std::{borrow::Cow, sync::OnceLock};

use regex::{self, Regex};
use serde::{Deserialize, Serialize, Serializer};
use serde_json::Number;

/// Generate the `title` and `description` setters shared by every schema
/// builder. The `with_default` setter is intentionally excluded — its argument
/// type differs per schema kind.
macro_rules! schema_title_description {
    () => {
        /// Set the human-readable title shown for this field.
        #[must_use]
        pub fn title(mut self, title: impl Into<Cow<'static, str>>) -> Self {
            self.title = Some(title.into());
            self
        }

        /// Set the description shown alongside this field.
        #[must_use]
        pub fn description(mut self, description: impl Into<Cow<'static, str>>) -> Self {
            self.description = Some(description.into());
            self
        }
    };
}

macro_rules! numeric_schema {
    (
        $(#[$meta:meta])* $name:ident, $type_const:ident,
        |$value:ident: $type:ty| $number:expr
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
        #[serde(rename_all = "camelCase")]
        pub struct $name {
            /// JSON Schema `type` tag, serialized as a literal.
            #[serde(rename = "type")]
            pub type_: $type_const,
            /// Human-readable field title.
            #[serde(skip_serializing_if = "Option::is_none")]
            pub title: Option<Cow<'static, str>>,
            /// Field description.
            #[serde(skip_serializing_if = "Option::is_none")]
            pub description: Option<Cow<'static, str>>,
            /// Inclusive lower bound.
            #[serde(skip_serializing_if = "Option::is_none")]
            pub minimum: Option<Number>,
            /// Inclusive upper bound.
            #[serde(skip_serializing_if = "Option::is_none")]
            pub maximum: Option<Number>,
            /// Value pre-filled for the user.
            #[serde(skip_serializing_if = "Option::is_none")]
            pub default: Option<Number>,
        }

        impl Default for $name {
            fn default() -> Self {
                Self {
                    type_: $type_const,
                    title: None,
                    description: None,
                    minimum: None,
                    maximum: None,
                    default: None,
                }
            }
        }

        impl $name {
            /// Unconstrained schema.
            pub fn new() -> Self {
                Self::default()
            }

            /// Set the inclusive `minimum..=maximum` bounds.
            ///
            /// # Errors
            ///
            /// Returns `"minimum must be <= maximum"` when `min > max`.
            pub fn range(self, min: $type, max: $type) -> Result<Self, &'static str> {
                if min > max {
                    return Err("minimum must be <= maximum");
                }
                Ok(self.minimum(min).maximum(max))
            }

            /// Set the inclusive lower bound.
            #[must_use]
            pub fn minimum(mut self, $value: $type) -> Self {
                self.minimum = $number;
                self
            }

            /// Set the inclusive upper bound.
            #[must_use]
            pub fn maximum(mut self, $value: $type) -> Self {
                self.maximum = $number;
                self
            }

            schema_title_description!();

            /// Set the value pre-filled for the user.
            #[must_use]
            pub fn with_default(mut self, $value: $type) -> Self {
                self.default = $number;
                self
            }
        }
    };
}

macro_rules! string_format_constructors {
    ($($name:ident => $format:ident),+ $(,)?) => {
        $(
            #[doc = concat!("String schema with [`StringFormat::", stringify!($format), "`].")]
            pub fn $name() -> Self {
                Self {
                    format: Some(StringFormat::$format),
                    ..Default::default()
                }
            }
        )+
    };
}

pub(crate) use schema_title_description;

use super::EnumSchema;
use crate::protocol::version::const_string;

const_string!(ObjectTypeConst = "object");
const_string!(StringTypeConst = "string");
const_string!(NumberTypeConst = "number");
const_string!(IntegerTypeConst = "integer");
const_string!(BooleanTypeConst = "boolean");
const_string!(EnumTypeConst = "string");
const_string!(ArrayTypeConst = "array");

/// Primitive schema definition for elicitation properties.
///
/// According to MCP 2025-11-25 specification, elicitation schemas must have
/// properties of primitive types only (string, number, integer, boolean, enum).
///
/// The spec unions overlap, so decoding picks the variant by `type`, then by the
/// most specific member present: `oneOf`, `enum` with `enumNames`, `enum` for
/// strings; `items.anyOf`, `items.enum` for arrays. Unknown members are ignored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum PrimitiveSchema {
    /// Enum property: a string with `oneOf` or `enum`, or an array of options
    Enum(EnumSchema),
    /// Free-text string property
    String(StringSchema),
    /// Number property (`"type": "number"`)
    Number(NumberSchema),
    /// Integer property (`"type": "integer"`)
    Integer(IntegerSchema),
    /// Boolean property
    Boolean(BooleanSchema),
}

/// String format types allowed by the MCP specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StringFormat {
    /// Email address format
    Email,
    /// URI format
    Uri,
    /// Date format (YYYY-MM-DD)
    Date,
    /// Date-time format (RFC 3339)
    DateTime,
}

/// Regex a string value must match, a `pattern` extension the spec does not define.
///
/// Equality and serialization use the source; the regex compiles once, on first use.
#[derive(Debug, Clone)]
pub struct Pattern {
    source: Cow<'static, str>,
    regex: OnceLock<Result<Regex, regex::Error>>,
}

impl Pattern {
    /// Pattern for the regex `source`.
    pub fn new(source: impl Into<Cow<'static, str>>) -> Self {
        Self {
            source: source.into(),
            regex: OnceLock::new(),
        }
    }

    /// The regex source.
    pub fn as_str(&self) -> &str {
        &self.source
    }

    /// The compiled regex, or why the source does not compile.
    pub(super) fn regex(&self) -> Result<&Regex, &regex::Error> {
        self.regex.get_or_init(|| Regex::new(&self.source)).as_ref()
    }
}

impl PartialEq for Pattern {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source
    }
}

impl Eq for Pattern {}

impl Serialize for Pattern {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.source)
    }
}

/// Schema definition for string properties.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StringSchema {
    /// JSON Schema `type` tag, serialized as `"string"`.
    #[serde(rename = "type")]
    pub type_: StringTypeConst,
    /// Human-readable field title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<Cow<'static, str>>,
    /// Field description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Cow<'static, str>>,
    /// Minimum length in characters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<u32>,
    /// Maximum length in characters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<u32>,
    /// Required string format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<StringFormat>,
    /// Regular expression the value must match.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<Pattern>,
    /// Value pre-filled for the user.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<Cow<'static, str>>,
}

impl Default for StringSchema {
    fn default() -> Self {
        Self {
            type_: StringTypeConst,
            title: None,
            description: None,
            min_length: None,
            max_length: None,
            format: None,
            pattern: None,
            default: None,
        }
    }
}

impl StringSchema {
    /// Unconstrained string schema.
    pub fn new() -> Self {
        Self::default()
    }

    string_format_constructors! {
        email => Email,
        uri => Uri,
        date => Date,
        date_time => DateTime,
    }

    schema_title_description!();

    /// Set the inclusive `min_length..=max_length` bounds.
    ///
    /// # Errors
    ///
    /// Returns `"min_length must be <= max_length"` when `min > max`.
    pub fn length(mut self, min: u32, max: u32) -> Result<Self, &'static str> {
        if min > max {
            return Err("min_length must be <= max_length");
        }
        self.min_length = Some(min);
        self.max_length = Some(max);
        Ok(self)
    }

    fluent_setter!(
        /// Set the minimum length in characters.
        pub fn min_length(min: u32) => some min_length
    );
    fluent_setter!(
        /// Set the maximum length in characters.
        pub fn max_length(max: u32) => some max_length
    );
    fluent_setter!(
        /// Set the required string format.
        pub fn format(format: StringFormat) => some format
    );
    /// Set the regular expression the value must match.
    #[must_use]
    pub fn pattern(mut self, pattern: impl Into<Cow<'static, str>>) -> Self {
        self.pattern = Some(Pattern::new(pattern));
        self
    }

    fluent_setter!(
        /// Set the value pre-filled for the user.
        pub fn with_default(default: impl Into<Cow<'static, str>>) => some_into default
    );
}

numeric_schema!(
    /// Schema definition for number properties; non-finite builder values have no JSON form and leave the field unset.
    NumberSchema,
    NumberTypeConst,
    |value: f64| Number::from_f64(value)
);

numeric_schema!(
    /// Schema definition for integer properties; bounds decode as any JSON number, as in the spec.
    IntegerSchema,
    IntegerTypeConst,
    |value: i64| Some(value.into())
);

/// Schema definition for boolean properties.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BooleanSchema {
    /// JSON Schema `type` tag, serialized as `"boolean"`.
    #[serde(rename = "type")]
    pub type_: BooleanTypeConst,
    /// Human-readable field title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<Cow<'static, str>>,
    /// Field description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Cow<'static, str>>,
    /// Value pre-filled for the user.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<bool>,
}

impl Default for BooleanSchema {
    fn default() -> Self {
        Self {
            type_: BooleanTypeConst,
            title: None,
            description: None,
            default: None,
        }
    }
}

impl BooleanSchema {
    /// Boolean schema without a default.
    pub fn new() -> Self {
        Self::default()
    }

    schema_title_description!();

    fluent_setter!(
        /// Set the value pre-filled for the user.
        pub fn with_default(default: bool) => some default
    );
}
