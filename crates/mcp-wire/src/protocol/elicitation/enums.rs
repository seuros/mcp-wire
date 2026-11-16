use std::borrow::Cow;

use serde::Serialize;

use super::{ArrayTypeConst, StringTypeConst, schema_title_description};

macro_rules! define_single_select_schema {
    (
        $(#[$meta:meta])*
        $name:ident {
            $(#[$values_meta:meta])*
            $values:ident: $values_type:ty
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
        #[serde(rename_all = "camelCase")]
        pub struct $name {
            /// JSON Schema `type` tag, serialized as `"string"`.
            #[serde(rename = "type")]
            pub type_: StringTypeConst,
            $(#[$values_meta])*
            pub $values: $values_type,
            /// Human-readable field title.
            #[serde(skip_serializing_if = "Option::is_none")]
            pub title: Option<Cow<'static, str>>,
            /// Field description.
            #[serde(skip_serializing_if = "Option::is_none")]
            pub description: Option<Cow<'static, str>>,
            /// Option value pre-selected for the user.
            #[serde(skip_serializing_if = "Option::is_none")]
            pub default: Option<String>,
        }

        impl $name {
            /// Schema offering `values`, with no title, description, or default.
            pub fn new(values: impl Into<$values_type>) -> Self {
                Self {
                    type_: StringTypeConst,
                    $values: values.into(),
                    title: None,
                    description: None,
                    default: None,
                }
            }

            schema_title_description!();

            /// Set the option value pre-selected for the user.
            #[must_use]
            pub fn with_default(mut self, default: impl Into<String>) -> Self {
                self.default = Some(default.into());
                self
            }
        }
    };
}

macro_rules! define_multi_select_schema {
    (
        $(#[$meta:meta])*
        $name:ident {
            items: $items_type:ty,
            input: $input_type:ty,
            build: $build:expr
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
        #[serde(rename_all = "camelCase")]
        pub struct $name {
            /// JSON Schema `type` tag, serialized as `"array"`.
            #[serde(rename = "type")]
            pub type_: ArrayTypeConst,
            /// Schema of the selectable options.
            pub items: $items_type,
            /// Minimum number of selected options.
            #[serde(skip_serializing_if = "Option::is_none")]
            pub min_items: Option<u32>,
            /// Maximum number of selected options.
            #[serde(skip_serializing_if = "Option::is_none")]
            pub max_items: Option<u32>,
            /// Human-readable field title.
            #[serde(skip_serializing_if = "Option::is_none")]
            pub title: Option<Cow<'static, str>>,
            /// Field description.
            #[serde(skip_serializing_if = "Option::is_none")]
            pub description: Option<Cow<'static, str>>,
            /// Option values pre-selected for the user.
            #[serde(skip_serializing_if = "Option::is_none")]
            pub default: Option<Vec<String>>,
        }

        impl $name {
            /// Schema offering `input` with no selection-count bounds.
            pub fn new(input: impl Into<$input_type>) -> Self {
                Self {
                    type_: ArrayTypeConst,
                    items: ($build)(input.into()),
                    min_items: None,
                    max_items: None,
                    title: None,
                    description: None,
                    default: None,
                }
            }

            schema_title_description!();

            /// Set the option values pre-selected for the user.
            #[must_use]
            pub fn with_default(mut self, default: impl Into<Vec<String>>) -> Self {
                self.default = Some(default.into());
                self
            }

            /// Set the minimum number of selected options.
            #[must_use]
            pub const fn min_items(mut self, min: u32) -> Self {
                self.min_items = Some(min);
                self
            }

            /// Set the maximum number of selected options.
            #[must_use]
            pub const fn max_items(mut self, max: u32) -> Self {
                self.max_items = Some(max);
                self
            }

            /// Set both selection-count bounds; `min > max` is not checked.
            #[must_use]
            pub const fn items_range(mut self, min: u32, max: u32) -> Self {
                self.min_items = Some(min);
                self.max_items = Some(max);
                self
            }
        }
    };
}

macro_rules! titled_from_pairs {
    ($name:ident) => {
        impl $name {
            /// Create from tuples of (value, title)
            pub fn from_pairs<I, V, T>(pairs: I) -> Self
            where
                I: IntoIterator<Item = (V, T)>,
                V: Into<String>,
                T: Into<String>,
            {
                Self::new(
                    pairs
                        .into_iter()
                        .map(|(value, title)| EnumOption::new(value, title))
                        .collect::<Vec<_>>(),
                )
            }
        }
    };
}

/// Option for titled enum variants (const value + display title)
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EnumOption {
    /// The actual enum value
    #[serde(rename = "const")]
    pub const_value: String,
    /// Display title for this option
    pub title: String,
}

impl EnumOption {
    /// Option with wire value `value` displayed as `title`.
    pub fn new(value: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            const_value: value.into(),
            title: title.into(),
        }
    }
}

/// Items schema for untitled multi-select (array of strings with enum constraint)
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UntitledEnumItems {
    /// JSON Schema `type` tag, serialized as `"string"`.
    #[serde(rename = "type")]
    pub type_: StringTypeConst,
    /// Allowed values.
    #[serde(rename = "enum")]
    pub enum_values: Vec<String>,
}

/// Items schema for titled multi-select (anyOf with const/title pairs)
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TitledEnumItems {
    /// Allowed values with their display titles.
    pub any_of: Vec<EnumOption>,
}

define_single_select_schema! {
    /// Schema for single-selection enum without display titles
    UntitledSingleSelectEnumSchema {
        /// Allowed values.
        #[serde(rename = "enum")]
        enum_values: Vec<String>
    }
}

define_single_select_schema! {
    /// Schema for single-selection enum with display titles for each option
    TitledSingleSelectEnumSchema {
        /// Allowed values with their display titles.
        one_of: Vec<EnumOption>
    }
}

titled_from_pairs!(TitledSingleSelectEnumSchema);

define_multi_select_schema! {
    /// Schema for multi-selection enum without display titles
    UntitledMultiSelectEnumSchema {
        items: UntitledEnumItems,
        input: Vec<String>,
        build: |values| UntitledEnumItems {
                type_: StringTypeConst,
                enum_values: values,
            }
    }
}

define_multi_select_schema! {
    /// Schema for multi-selection enum with display titles for each option
    TitledMultiSelectEnumSchema {
        items: TitledEnumItems,
        input: Vec<EnumOption>,
        build: |options| TitledEnumItems { any_of: options }
    }
}

titled_from_pairs!(TitledMultiSelectEnumSchema);

/// Legacy enum schema with parallel enum/enumNames arrays
///
/// Deprecated: use `TitledSingleSelectEnumSchema` instead.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyTitledEnumSchema {
    /// JSON Schema `type` tag, serialized as `"string"`.
    #[serde(rename = "type")]
    pub type_: StringTypeConst,
    /// Allowed values.
    #[serde(rename = "enum")]
    pub enum_values: Vec<String>,
    /// Display names for enum values (parallel array, same length as `enum_values`)
    pub enum_names: Vec<String>,
    /// Human-readable field title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<Cow<'static, str>>,
    /// Field description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Cow<'static, str>>,
    /// Value pre-selected for the user.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
}

impl LegacyTitledEnumSchema {
    /// Schema from parallel `values` and display `names`.
    ///
    /// # Errors
    ///
    /// Returns `"enum values and names must have same length"` when they differ.
    pub fn new(values: Vec<String>, names: Vec<String>) -> Result<Self, &'static str> {
        if values.len() != names.len() {
            return Err("enum values and names must have same length");
        }
        Ok(Self::from_parallel(values, names))
    }

    /// Schema from parallel `values` and `names` of equal length.
    const fn from_parallel(values: Vec<String>, names: Vec<String>) -> Self {
        Self {
            type_: StringTypeConst,
            enum_values: values,
            enum_names: names,
            title: None,
            description: None,
            default: None,
        }
    }

    /// Create from tuples of (value, name)
    pub fn from_pairs<I, V, N>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (V, N)>,
        V: Into<String>,
        N: Into<String>,
    {
        let (values, names): (Vec<_>, Vec<_>) =
            pairs.into_iter().map(|(v, n)| (v.into(), n.into())).unzip();
        Self::from_parallel(values, names)
    }

    schema_title_description!();

    fluent_setter!(
        /// Set the value pre-selected for the user.
        pub fn with_default(default: impl Into<String>) => some_into default
    );
}

/// Union of all enum schema variants for elicitation, decoded like [`super::PrimitiveSchema`]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum EnumSchema {
    /// Single-select with titled options (uses oneOf)
    TitledSingleSelect(TitledSingleSelectEnumSchema),
    /// Multi-select with titled options (uses items.anyOf)
    TitledMultiSelect(TitledMultiSelectEnumSchema),
    /// Multi-select without titles (type: "array")
    UntitledMultiSelect(UntitledMultiSelectEnumSchema),
    /// Single-select without titles (simple enum array)
    UntitledSingleSelect(UntitledSingleSelectEnumSchema),
    /// Legacy format with parallel enumNames array (deprecated)
    Legacy(LegacyTitledEnumSchema),
}

impl EnumSchema {
    /// Create a simple single-select enum from string values
    pub fn single_select<I, S>(values: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self::UntitledSingleSelect(UntitledSingleSelectEnumSchema::new(
            values.into_iter().map(Into::into).collect::<Vec<_>>(),
        ))
    }

    /// Create a single-select enum with display titles
    pub fn single_select_titled<I, V, T>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (V, T)>,
        V: Into<String>,
        T: Into<String>,
    {
        Self::TitledSingleSelect(TitledSingleSelectEnumSchema::from_pairs(pairs))
    }

    /// Create a simple multi-select enum from string values
    pub fn multi_select<I, S>(values: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self::UntitledMultiSelect(UntitledMultiSelectEnumSchema::new(
            values.into_iter().map(Into::into).collect::<Vec<_>>(),
        ))
    }

    /// Create a multi-select enum with display titles
    pub fn multi_select_titled<I, V, T>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (V, T)>,
        V: Into<String>,
        T: Into<String>,
    {
        Self::TitledMultiSelect(TitledMultiSelectEnumSchema::from_pairs(pairs))
    }

    /// Add title to the enum schema
    #[must_use]
    pub fn title(self, title: impl Into<Cow<'static, str>>) -> Self {
        let title = title.into();
        match self {
            Self::TitledSingleSelect(s) => Self::TitledSingleSelect(s.title(title)),
            Self::TitledMultiSelect(s) => Self::TitledMultiSelect(s.title(title)),
            Self::UntitledMultiSelect(s) => Self::UntitledMultiSelect(s.title(title)),
            Self::UntitledSingleSelect(s) => Self::UntitledSingleSelect(s.title(title)),
            Self::Legacy(s) => Self::Legacy(s.title(title)),
        }
    }

    /// Add description to the enum schema
    #[must_use]
    pub fn description(self, description: impl Into<Cow<'static, str>>) -> Self {
        let description = description.into();
        match self {
            Self::TitledSingleSelect(s) => Self::TitledSingleSelect(s.description(description)),
            Self::TitledMultiSelect(s) => Self::TitledMultiSelect(s.description(description)),
            Self::UntitledMultiSelect(s) => Self::UntitledMultiSelect(s.description(description)),
            Self::UntitledSingleSelect(s) => Self::UntitledSingleSelect(s.description(description)),
            Self::Legacy(s) => Self::Legacy(s.description(description)),
        }
    }
}
