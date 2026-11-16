use std::cmp::Ordering;

use fluent_uri::Uri;
use jiff::{Timestamp, civil::Date};
use serde_json::{Number, Value};
use thiserror::Error;

use super::{
    BooleanSchema, ElicitationSchema, EnumSchema, LegacyTitledEnumSchema, PrimitiveSchema,
    StringFormat, StringSchema, UntitledSingleSelectEnumSchema,
};
use crate::protocol::types::{CreateElicitationRequest, CreateElicitationResult};
use crate::protocol::version::ProtocolVersion;

/// Validation errors for form-mode elicitation results.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ElicitationValidationError {
    /// The client accepted the request but omitted form content.
    #[error("accepted elicitation result must include content")]
    AcceptedWithoutContent,
    /// Decline and cancel responses must not include form content.
    #[error("decline and cancel elicitation results must omit content")]
    NonAcceptWithContent,
    /// The form content was not a JSON object.
    #[error("elicitation content must be an object")]
    ContentNotObject,
    /// A required field was not present in the submitted form content.
    #[error("missing required elicitation field '{field}'")]
    MissingRequired {
        /// Missing field name.
        field: String,
    },
    /// A submitted value did not match the requested field schema.
    #[error("invalid elicitation field '{field}': {reason}")]
    InvalidField {
        /// Field name that failed validation.
        field: String,
        /// Human-readable validation failure.
        reason: String,
    },
    /// The requested schema itself is invalid or unsupported.
    #[error("invalid elicitation schema: {0}")]
    InvalidSchema(String),
}

impl PrimitiveSchema {
    pub(super) fn validate_value(
        &self,
        field: &str,
        value: &Value,
    ) -> Result<(), ElicitationValidationError> {
        match self {
            Self::Enum(schema) => schema.validate_value(field, value),
            Self::String(schema) => schema.validate_value(field, value),
            Self::Number(schema) => validate_number(
                field,
                value,
                false,
                schema.minimum.as_ref(),
                schema.maximum.as_ref(),
            ),
            Self::Integer(schema) => validate_number(
                field,
                value,
                true,
                schema.minimum.as_ref(),
                schema.maximum.as_ref(),
            ),
            Self::Boolean(_) => BooleanSchema::validate_value(field, value),
        }
    }
}

impl StringSchema {
    fn validate_value(&self, field: &str, value: &Value) -> Result<(), ElicitationValidationError> {
        let value = value
            .as_str()
            .ok_or_else(|| invalid_field(field, "expected string"))?;

        if let Some(min) = self.min_length
            && value.chars().count() < min as usize
        {
            return Err(invalid_field(
                field,
                format!("must be at least {min} characters"),
            ));
        }

        if let Some(max) = self.max_length
            && value.chars().count() > max as usize
        {
            return Err(invalid_field(
                field,
                format!("must be at most {max} characters"),
            ));
        }

        if let Some(format) = self.format {
            validate_string_format(field, value, format)?;
        }

        if let Some(pattern) = &self.pattern {
            let regex = pattern.regex().map_err(|err| {
                ElicitationValidationError::InvalidSchema(format!(
                    "invalid pattern for field '{field}': {err}"
                ))
            })?;
            if !regex.is_match(value) {
                return Err(invalid_field(field, "does not match required pattern"));
            }
        }

        Ok(())
    }
}

impl BooleanSchema {
    fn validate_value(field: &str, value: &Value) -> Result<(), ElicitationValidationError> {
        if value.is_boolean() {
            Ok(())
        } else {
            Err(invalid_field(field, "expected boolean"))
        }
    }
}

impl EnumSchema {
    fn validate_value(&self, field: &str, value: &Value) -> Result<(), ElicitationValidationError> {
        match self {
            Self::TitledSingleSelect(schema) => validate_single_enum(
                field,
                value,
                schema
                    .one_of
                    .iter()
                    .map(|option| option.const_value.as_str()),
            ),
            Self::TitledMultiSelect(schema) => validate_multi_enum(
                field,
                value,
                &schema
                    .items
                    .any_of
                    .iter()
                    .map(|option| option.const_value.as_str()),
                schema.min_items,
                schema.max_items,
            ),
            Self::UntitledMultiSelect(schema) => validate_multi_enum(
                field,
                value,
                &schema.items.enum_values.iter().map(String::as_str),
                schema.min_items,
                schema.max_items,
            ),
            Self::UntitledSingleSelect(UntitledSingleSelectEnumSchema { enum_values, .. })
            | Self::Legacy(LegacyTitledEnumSchema { enum_values, .. }) => {
                validate_single_enum(field, value, enum_values.iter().map(String::as_str))
            }
        }
    }
}

impl CreateElicitationRequest {
    /// Build a form-mode elicitation request from a typed schema.
    #[expect(
        clippy::missing_panics_doc,
        reason = "ElicitationSchema has only string map keys and plain fields; \
                  serializing it to a Value cannot fail"
    )]
    pub fn form(message: impl Into<String>, requested_schema: ElicitationSchema) -> Self {
        Self {
            message: message.into(),
            requested_schema: serde_json::to_value(requested_schema)
                .expect("ElicitationSchema has string map keys and no fallible Serialize impl"),
        }
    }

    /// Build a confirmation request with a required `confirmed` boolean.
    pub fn confirm(message: impl Into<String>) -> Self {
        Self::form(message, ElicitationSchema::confirm())
    }

    /// Validate a client response, under `version`, against this request's form schema.
    ///
    /// # Errors
    ///
    /// Same as [`validate_elicitation_response`] with this request's
    /// `requested_schema`.
    pub fn validate_result(
        &self,
        result: &CreateElicitationResult,
        version: ProtocolVersion,
    ) -> Result<(), ElicitationValidationError> {
        validate_elicitation_response(&self.requested_schema, result, version)
    }
}

/// Validate a client elicitation result, under `version`, against a serialized requested schema.
///
/// # Errors
///
/// - [`ElicitationValidationError::InvalidSchema`] when `requested_schema` is
///   not a supported form schema.
/// - Any error from [`ElicitationSchema::validate_result`].
pub fn validate_elicitation_response(
    requested_schema: &Value,
    result: &CreateElicitationResult,
    version: ProtocolVersion,
) -> Result<(), ElicitationValidationError> {
    parse_elicitation_schema(requested_schema)?.validate_result(result, version)
}

/// Retain only the typed validation contract, not a duplicate wire JSON tree.
///
/// # Errors
///
/// When `requested_schema` is not a schema the protocol allows for elicitation.
pub fn parse_elicitation_schema(
    requested_schema: &Value,
) -> Result<ElicitationSchema, ElicitationValidationError> {
    serde::Deserialize::deserialize(requested_schema).map_err(|err| {
        ElicitationValidationError::InvalidSchema(format!(
            "requestedSchema is not a supported form schema: {err}"
        ))
    })
}

/// Check an undeclared field's value against `ElicitResult.content`: a
/// string, number, or boolean (2025-06-18 schema.ts:1556), or since
/// 2025-11-25 a string array (schema.ts:2485).
pub(super) fn validate_elicitation_content_value(
    field: &str,
    value: &Value,
    version: ProtocolVersion,
) -> Result<(), ElicitationValidationError> {
    if value.is_string() || value.is_number() || value.is_boolean() {
        return Ok(());
    }
    if version < ProtocolVersion::V2025_11_25 {
        return Err(invalid_field(field, "expected string, number, or boolean"));
    }

    if let Some(values) = value.as_array()
        && values.iter().all(Value::is_string)
    {
        return Ok(());
    }

    Err(invalid_field(
        field,
        "expected string, number, boolean, or string array",
    ))
}

fn validate_string_format(
    field: &str,
    value: &str,
    format: StringFormat,
) -> Result<(), ElicitationValidationError> {
    match format {
        StringFormat::Email if !is_valid_email(value) => {
            Err(invalid_field(field, "expected email format"))
        }
        StringFormat::Uri if Uri::parse(value).is_err() => {
            Err(invalid_field(field, "expected URI format"))
        }
        StringFormat::Date if !is_full_date(value) => {
            Err(invalid_field(field, "expected date format YYYY-MM-DD"))
        }
        StringFormat::DateTime if !is_date_time(value) => {
            Err(invalid_field(field, "expected RFC 3339 date-time format"))
        }
        _ => Ok(()),
    }
}

/// Check a JSON number, integral for `integer` (zero fractional part, as in JSON Schema), against inclusive bounds.
fn validate_number(
    field: &str,
    value: &Value,
    integer: bool,
    minimum: Option<&Number>,
    maximum: Option<&Number>,
) -> Result<(), ElicitationValidationError> {
    let value = value
        .as_number()
        .filter(|value| !integer || value.as_f64().is_some_and(|value| value.fract() == 0.0))
        .ok_or_else(|| {
            invalid_field(
                field,
                if integer {
                    "expected integer"
                } else {
                    "expected number"
                },
            )
        })?;

    if let Some(min) = minimum
        && order(value, min).is_some_and(Ordering::is_lt)
    {
        return Err(invalid_field(field, format!("must be >= {min}")));
    }

    if let Some(max) = maximum
        && order(value, max).is_some_and(Ordering::is_gt)
    {
        return Err(invalid_field(field, format!("must be <= {max}")));
    }

    Ok(())
}

/// Order two JSON numbers, exactly when both fit `i64`, else as `f64`.
fn order(a: &Number, b: &Number) -> Option<Ordering> {
    match (a.as_i64(), b.as_i64()) {
        (Some(a), Some(b)) => Some(a.cmp(&b)),
        _ => a.as_f64()?.partial_cmp(&b.as_f64()?),
    }
}

/// Whether `value` follows `shape`, where `0` is any ASCII digit and letters match in either case.
fn has_shape(value: &str, shape: &str) -> bool {
    value.len() == shape.len()
        && value.bytes().zip(shape.bytes()).all(|(byte, expected)| {
            if expected == b'0' {
                byte.is_ascii_digit()
            } else {
                byte.eq_ignore_ascii_case(&expected)
            }
        })
}

/// RFC 3339 `full-date` naming a real day.
fn is_full_date(value: &str) -> bool {
    has_shape(value, "0000-00-00") && Date::strptime("%Y-%m-%d", value).is_ok()
}

/// RFC 3339 `date-time` (`full-date "T" HH:MM:SS [.frac] ("Z" / ±HH:MM)`) naming a real instant.
fn is_date_time(value: &str) -> bool {
    let Some((head, mut offset)) = value.split_at_checked(19) else {
        return false;
    };
    if let Some(fraction) = offset.strip_prefix('.') {
        offset = fraction.trim_start_matches(|ch: char| ch.is_ascii_digit());
        if offset.len() == fraction.len() {
            return false;
        }
    }
    has_shape(head, "0000-00-00T00:00:00")
        && (offset.eq_ignore_ascii_case("z")
            || offset
                .strip_prefix(['+', '-'])
                .is_some_and(|offset| has_shape(offset, "00:00")))
        && value.parse::<Timestamp>().is_ok()
}

fn validate_single_enum<'a>(
    field: &str,
    value: &Value,
    allowed: impl IntoIterator<Item = &'a str>,
) -> Result<(), ElicitationValidationError> {
    let value = value
        .as_str()
        .ok_or_else(|| invalid_field(field, "expected string enum value"))?;
    if allowed.into_iter().any(|allowed| allowed == value) {
        Ok(())
    } else {
        Err(invalid_field(field, "is not one of the allowed values"))
    }
}

fn validate_multi_enum<'a>(
    field: &str,
    value: &Value,
    allowed: &(impl Iterator<Item = &'a str> + Clone),
    min_items: Option<u32>,
    max_items: Option<u32>,
) -> Result<(), ElicitationValidationError> {
    let values = value
        .as_array()
        .ok_or_else(|| invalid_field(field, "expected array of string enum values"))?;

    if let Some(min) = min_items
        && values.len() < min as usize
    {
        return Err(invalid_field(
            field,
            format!("must include at least {min} items"),
        ));
    }

    if let Some(max) = max_items
        && values.len() > max as usize
    {
        return Err(invalid_field(
            field,
            format!("must include at most {max} items"),
        ));
    }

    for value in values {
        let Some(value) = value.as_str() else {
            return Err(invalid_field(field, "expected only string enum values"));
        };
        if !allowed.clone().any(|allowed| allowed == value) {
            return Err(invalid_field(
                field,
                "contains a value outside the allowed set",
            ));
        }
    }

    Ok(())
}

fn invalid_field(field: &str, reason: impl Into<String>) -> ElicitationValidationError {
    ElicitationValidationError::InvalidField {
        field: field.to_string(),
        reason: reason.into(),
    }
}

fn is_valid_email(value: &str) -> bool {
    let mut parts = value.split('@');
    matches!(
        (parts.next(), parts.next(), parts.next()),
        (Some(local), Some(domain), None)
            if !local.is_empty() && domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.')
    )
}
