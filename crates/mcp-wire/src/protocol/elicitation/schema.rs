use std::{borrow::Cow, collections::BTreeMap};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    BooleanSchema, ElicitationValidationError, EnumSchema, IntegerSchema, LegacyTitledEnumSchema,
    NumberSchema, ObjectTypeConst, PrimitiveSchema, StringSchema, UntitledSingleSelectEnumSchema,
    decode::deserialize_properties, schema_title_description, validate_elicitation_content_value,
};
use crate::protocol::types::{CreateElicitationResult, ElicitationAction};
use crate::protocol::version::{ProtocolVersion, UnsupportedByVersion};

/// Type-safe elicitation schema for requesting structured user input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElicitationSchema {
    /// JSON Schema dialect URI (`$schema`).
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<Cow<'static, str>>,
    /// JSON Schema `type` tag, serialized as `"object"`.
    #[serde(rename = "type")]
    pub type_: ObjectTypeConst,
    /// Human-readable form title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<Cow<'static, str>>,
    /// Form fields by name.
    #[serde(deserialize_with = "deserialize_properties")]
    pub properties: BTreeMap<String, PrimitiveSchema>,
    /// Names of fields the user must fill in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
    /// Form description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Cow<'static, str>>,
}

impl ElicitationSchema {
    /// Schema with `properties`, all optional, without title or description.
    pub const fn new(properties: BTreeMap<String, PrimitiveSchema>) -> Self {
        Self {
            schema: None,
            type_: ObjectTypeConst,
            title: None,
            properties,
            required: None,
            description: None,
        }
    }

    /// Start an empty [`ElicitationSchemaBuilder`].
    pub fn builder() -> ElicitationSchemaBuilder {
        ElicitationSchemaBuilder::new()
    }

    /// Build a simple confirmation schema with a required `confirmed` boolean.
    pub fn confirm() -> Self {
        Self::confirm_with_field("confirmed")
    }

    /// Build a confirmation schema using a custom boolean field name.
    pub fn confirm_with_field(field_name: impl Into<String>) -> Self {
        let field = field_name.into();
        Self {
            title: Some(Cow::Borrowed("Confirmation")),
            required: Some(vec![field.clone()]),
            ..Self::new(BTreeMap::from([(
                field,
                PrimitiveSchema::Boolean(BooleanSchema::new()),
            )]))
        }
    }

    /// This schema as `version` defines `requestedSchema`.
    ///
    /// 2025-11-25 takes it as built. 2025-06-18 (schema.ts 2025-06-18:1469-1536)
    /// takes titled options as `enum` with `enumNames`, and none of the
    /// members it lacks: `$schema`, the form's `title` and `description`,
    /// `pattern`, and every `default` but a boolean's (:1524).
    ///
    /// # Errors
    ///
    /// [`UnsupportedByVersion`] naming the first multi-select property, which
    /// 2025-06-18 has no form for.
    pub fn for_version(
        &self,
        version: ProtocolVersion,
    ) -> Result<Cow<'_, Self>, UnsupportedByVersion> {
        if version >= ProtocolVersion::V2025_11_25 {
            return Ok(Cow::Borrowed(self));
        }
        let properties = self
            .properties
            .iter()
            .map(|(name, schema)| {
                let schema = schema.to_2025_06_18().ok_or_else(|| UnsupportedByVersion {
                    feature: format!("multi-select enum property {name:?}"),
                    version,
                })?;
                Ok((name.clone(), schema))
            })
            .collect::<Result<_, _>>()?;
        Ok(Cow::Owned(Self {
            required: self.required.clone(),
            ..Self::new(properties)
        }))
    }

    /// Validate an accepted form response content object against this schema.
    ///
    /// # Errors
    ///
    /// - [`ElicitationValidationError::ContentNotObject`] when `content` is not
    ///   a JSON object.
    /// - [`ElicitationValidationError::MissingRequired`] for the first field in
    ///   `required` that `content` lacks.
    /// - [`ElicitationValidationError::InvalidField`] for the first value that
    ///   does not match its field schema; undeclared fields must be a string,
    ///   number, or boolean, or under 2025-11-25 a string array.
    /// - [`ElicitationValidationError::InvalidSchema`] when a string field's
    ///   `pattern` is not a valid regex.
    pub fn validate_content(
        &self,
        content: &Value,
        version: ProtocolVersion,
    ) -> Result<(), ElicitationValidationError> {
        let object = content
            .as_object()
            .ok_or(ElicitationValidationError::ContentNotObject)?;

        if let Some(required) = &self.required {
            for field in required {
                if !object.contains_key(field) {
                    return Err(ElicitationValidationError::MissingRequired {
                        field: field.clone(),
                    });
                }
            }
        }

        for (field, value) in object {
            if let Some(schema) = self.properties.get(field) {
                schema.validate_value(field, value)?;
            } else {
                validate_elicitation_content_value(field, value, version)?;
            }
        }

        Ok(())
    }

    /// Validate a full elicitation result, from a client that negotiated `version`, against this schema.
    ///
    /// # Errors
    ///
    /// - [`ElicitationValidationError::AcceptedWithoutContent`] when the user
    ///   accepted but `content` is absent.
    /// - [`ElicitationValidationError::NonAcceptWithContent`] when a declined
    ///   or cancelled result carries `content`.
    /// - Any error from [`Self::validate_content`] for accepted content.
    pub fn validate_result(
        &self,
        result: &CreateElicitationResult,
        version: ProtocolVersion,
    ) -> Result<(), ElicitationValidationError> {
        match result.action {
            ElicitationAction::Accept => {
                let content = result
                    .content
                    .as_ref()
                    .ok_or(ElicitationValidationError::AcceptedWithoutContent)?;
                self.validate_content(content, version)
            }
            ElicitationAction::Decline | ElicitationAction::Cancel => {
                if result.content.is_some() {
                    Err(ElicitationValidationError::NonAcceptWithContent)
                } else {
                    Ok(())
                }
            }
        }
    }
}

impl PrimitiveSchema {
    /// This property as 2025-06-18 defines it (schema.ts 2025-06-18:1485-1536);
    /// `None` for a multi-select, which it cannot express.
    fn to_2025_06_18(&self) -> Option<Self> {
        Some(match self {
            Self::String(schema) => Self::String(StringSchema {
                pattern: None,
                default: None,
                ..schema.clone()
            }),
            Self::Number(schema) => Self::Number(NumberSchema {
                default: None,
                ..schema.clone()
            }),
            Self::Integer(schema) => Self::Integer(IntegerSchema {
                default: None,
                ..schema.clone()
            }),
            Self::Boolean(schema) => Self::Boolean(schema.clone()),
            Self::Enum(EnumSchema::UntitledSingleSelect(schema)) => Self::Enum(
                EnumSchema::UntitledSingleSelect(UntitledSingleSelectEnumSchema {
                    default: None,
                    ..schema.clone()
                }),
            ),
            Self::Enum(EnumSchema::Legacy(schema)) => {
                Self::Enum(EnumSchema::Legacy(LegacyTitledEnumSchema {
                    default: None,
                    ..schema.clone()
                }))
            }
            // `oneOf` options as the parallel `enum` and `enumNames` (:1534-1535).
            Self::Enum(EnumSchema::TitledSingleSelect(schema)) => {
                Self::Enum(EnumSchema::Legacy(LegacyTitledEnumSchema {
                    title: schema.title.clone(),
                    description: schema.description.clone(),
                    ..LegacyTitledEnumSchema::from_pairs(
                        schema
                            .one_of
                            .iter()
                            .map(|option| (option.const_value.as_str(), option.title.as_str())),
                    )
                }))
            }
            Self::Enum(EnumSchema::TitledMultiSelect(_) | EnumSchema::UntitledMultiSelect(_)) => {
                return None;
            }
        })
    }
}

/// Fluent builder for constructing elicitation schemas.
#[derive(Debug, Default)]
pub struct ElicitationSchemaBuilder {
    /// JSON Schema dialect URI (`$schema`).
    pub schema: Option<Cow<'static, str>>,
    /// Form fields by name.
    pub properties: BTreeMap<String, PrimitiveSchema>,
    /// Names of fields the user must fill in, in insertion order.
    pub required: Vec<String>,
    /// Human-readable form title.
    pub title: Option<Cow<'static, str>>,
    /// Form description.
    pub description: Option<Cow<'static, str>>,
}

impl ElicitationSchemaBuilder {
    /// Builder with no fields.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an optional field, replacing any field with the same name.
    #[must_use]
    pub fn property(mut self, name: impl Into<String>, schema: PrimitiveSchema) -> Self {
        self.properties.insert(name.into(), schema);
        self
    }

    /// Add a field the user must fill in.
    #[must_use]
    pub fn required_property(mut self, name: impl Into<String>, schema: PrimitiveSchema) -> Self {
        let name_str = name.into();
        self.required.push(name_str.clone());
        self.properties.insert(name_str, schema);
        self
    }

    /// Add a required string field with the `email` format.
    #[must_use]
    pub fn required_email(self, name: impl Into<String>) -> Self {
        self.required_property(name, PrimitiveSchema::String(StringSchema::email()))
    }

    /// Add a required unconstrained string field.
    #[must_use]
    pub fn required_string(self, name: impl Into<String>) -> Self {
        self.required_property(name, PrimitiveSchema::String(StringSchema::new()))
    }

    /// Add an optional unconstrained string field.
    #[must_use]
    pub fn optional_string(self, name: impl Into<String>) -> Self {
        self.property(name, PrimitiveSchema::String(StringSchema::new()))
    }

    /// Add a required boolean field.
    #[must_use]
    pub fn required_bool(self, name: impl Into<String>) -> Self {
        self.required_property(name, PrimitiveSchema::Boolean(BooleanSchema::new()))
    }

    /// Add an optional boolean field pre-filled with `default`.
    #[must_use]
    pub fn optional_bool(self, name: impl Into<String>, default: bool) -> Self {
        self.property(
            name,
            PrimitiveSchema::Boolean(BooleanSchema::new().with_default(default)),
        )
    }

    /// Add a required number field bounded to `min..=max`.
    ///
    /// # Errors
    ///
    /// Returns `"minimum must be <= maximum"` when `min > max`.
    pub fn required_number(
        self,
        name: impl Into<String>,
        min: f64,
        max: f64,
    ) -> Result<Self, &'static str> {
        let schema = NumberSchema::new().range(min, max)?;
        Ok(self.required_property(name, PrimitiveSchema::Number(schema)))
    }

    /// Add an optional unbounded number field.
    #[must_use]
    pub fn optional_number(self, name: impl Into<String>) -> Self {
        self.property(name, PrimitiveSchema::Number(NumberSchema::new()))
    }

    /// Add a required integer field bounded to `min..=max`.
    ///
    /// # Errors
    ///
    /// Returns `"minimum must be <= maximum"` when `min > max`.
    pub fn required_integer(
        self,
        name: impl Into<String>,
        min: i64,
        max: i64,
    ) -> Result<Self, &'static str> {
        let schema = IntegerSchema::new().range(min, max)?;
        Ok(self.required_property(name, PrimitiveSchema::Integer(schema)))
    }

    /// Add an optional unbounded integer field.
    #[must_use]
    pub fn optional_integer(self, name: impl Into<String>) -> Self {
        self.property(name, PrimitiveSchema::Integer(IntegerSchema::new()))
    }

    /// Add a required single-select enum property
    #[must_use]
    pub fn required_enum<I, S>(self, name: impl Into<String>, values: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.required_property(
            name,
            PrimitiveSchema::Enum(EnumSchema::single_select(values)),
        )
    }

    /// Add an optional single-select enum property with a default value
    #[must_use]
    pub fn optional_enum<I, S>(self, name: impl Into<String>, values: I, default: S) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let values_vec: Vec<String> = values.into_iter().map(Into::into).collect();
        let schema = UntitledSingleSelectEnumSchema::new(values_vec).with_default(default);
        self.property(
            name,
            PrimitiveSchema::Enum(EnumSchema::UntitledSingleSelect(schema)),
        )
    }

    /// Add a required single-select enum with titled options
    #[must_use]
    pub fn required_enum_titled<I, V, T>(self, name: impl Into<String>, options: I) -> Self
    where
        I: IntoIterator<Item = (V, T)>,
        V: Into<String>,
        T: Into<String>,
    {
        self.required_property(
            name,
            PrimitiveSchema::Enum(EnumSchema::single_select_titled(options)),
        )
    }

    /// Add a required multi-select enum property
    #[must_use]
    pub fn required_multi_enum<I, S>(self, name: impl Into<String>, values: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.required_property(
            name,
            PrimitiveSchema::Enum(EnumSchema::multi_select(values)),
        )
    }

    /// Add an optional multi-select enum property
    #[must_use]
    pub fn optional_multi_enum<I, S>(self, name: impl Into<String>, values: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.property(
            name,
            PrimitiveSchema::Enum(EnumSchema::multi_select(values)),
        )
    }

    schema_title_description!();

    fluent_setter!(
        /// Set the JSON Schema dialect URI (`$schema`).
        pub fn schema_uri(schema: impl Into<Cow<'static, str>>) => some_into schema
    );

    /// Finish the schema; `required` is omitted when no field is required.
    ///
    /// # Errors
    ///
    /// Returns `"Required field does not exist in properties"` when a
    /// `required` name has no matching entry in `properties`.
    pub fn build(self) -> Result<ElicitationSchema, &'static str> {
        if self
            .required
            .iter()
            .any(|field| !self.properties.contains_key(field))
        {
            return Err("Required field does not exist in properties");
        }

        Ok(ElicitationSchema {
            schema: self.schema,
            type_: ObjectTypeConst,
            title: self.title,
            properties: self.properties,
            required: if self.required.is_empty() {
                None
            } else {
                Some(self.required)
            },
            description: self.description,
        })
    }
}
