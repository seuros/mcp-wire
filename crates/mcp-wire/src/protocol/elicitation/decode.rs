//! Spec-ordered decoding of elicitation property schemas.

use std::{borrow::Cow, collections::BTreeMap};

use serde::{
    Deserialize, Deserializer,
    de::{DeserializeOwned, Error as _},
};
use serde_json::{Map, Value};

use super::{
    ArrayTypeConst, BooleanSchema, BooleanTypeConst, EnumOption, EnumSchema, IntegerSchema,
    IntegerTypeConst, LegacyTitledEnumSchema, NumberSchema, NumberTypeConst, Pattern,
    PrimitiveSchema, StringSchema, StringTypeConst, TitledEnumItems, TitledMultiSelectEnumSchema,
    TitledSingleSelectEnumSchema, UntitledEnumItems, UntitledMultiSelectEnumSchema,
    UntitledSingleSelectEnumSchema,
};

impl<'de> Deserialize<'de> for PrimitiveSchema {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::decode(Value::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

impl<'de> Deserialize<'de> for EnumSchema {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match PrimitiveSchema::deserialize(deserializer)? {
            PrimitiveSchema::Enum(schema) => Ok(schema),
            _ => Err(D::Error::custom(
                "not an enum schema: no \"oneOf\", \"enum\" or array \"items\"",
            )),
        }
    }
}

/// Decode `requestedSchema.properties`, naming the property in each error.
pub(super) fn deserialize_properties<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<String, PrimitiveSchema>, D::Error> {
    Map::deserialize(deserializer)?
        .into_iter()
        .map(|(key, value)| match PrimitiveSchema::decode(value) {
            Ok(schema) => Ok((key, schema)),
            Err(err) => Err(D::Error::custom(format_args!("property {key:?}: {err}"))),
        })
        .collect()
}

impl PrimitiveSchema {
    /// Decode by `type`, then by the most specific discriminating member present.
    fn decode(value: Value) -> Result<Self, String> {
        let mut members = Members::object(value, "")?;
        let tag = members.map.remove("type");
        let Some(kind @ ("string" | "array" | "number" | "integer" | "boolean")) =
            tag.as_ref().and_then(Value::as_str)
        else {
            return Err(tag.map_or_else(
                || "missing \"type\"".to_owned(),
                |tag| format!("unknown \"type\" {tag}"),
            ));
        };
        let title = members.opt("title")?;
        let description = members.opt("description")?;
        Ok(match kind {
            "string" if members.has("oneOf") => Self::Enum(EnumSchema::TitledSingleSelect(
                TitledSingleSelectEnumSchema {
                    type_: StringTypeConst,
                    one_of: members.options("oneOf")?,
                    title,
                    description,
                    default: members.opt("default")?,
                },
            )),
            "string" if members.has("enum") && members.has("enumNames") => {
                Self::Enum(EnumSchema::Legacy(LegacyTitledEnumSchema {
                    type_: StringTypeConst,
                    enum_values: members.req("enum")?,
                    enum_names: members.req("enumNames")?,
                    title,
                    description,
                    default: members.opt("default")?,
                }))
            }
            "string" if members.has("enum") => Self::Enum(EnumSchema::UntitledSingleSelect(
                UntitledSingleSelectEnumSchema {
                    type_: StringTypeConst,
                    enum_values: members.req("enum")?,
                    title,
                    description,
                    default: members.opt("default")?,
                },
            )),
            "string" => Self::String(StringSchema {
                type_: StringTypeConst,
                title,
                description,
                min_length: members.opt("minLength")?,
                max_length: members.opt("maxLength")?,
                format: members.opt("format")?,
                pattern: members.opt::<String>("pattern")?.map(Pattern::new),
                default: members.opt("default")?,
            }),
            "array" => Self::Enum(members.multi_select(title, description)?),
            "number" => Self::Number(NumberSchema {
                type_: NumberTypeConst,
                title,
                description,
                minimum: members.opt("minimum")?,
                maximum: members.opt("maximum")?,
                default: members.opt("default")?,
            }),
            "integer" => Self::Integer(IntegerSchema {
                type_: IntegerTypeConst,
                title,
                description,
                minimum: members.opt("minimum")?,
                maximum: members.opt("maximum")?,
                default: members.opt("default")?,
            }),
            _ => Self::Boolean(BooleanSchema {
                type_: BooleanTypeConst,
                title,
                description,
                default: members.opt("default")?,
            }),
        })
    }
}

/// A schema object's members, each removed as it is decoded.
struct Members {
    map: Map<String, Value>,
    /// Error prefix locating the object inside its property; empty at the top.
    at: &'static str,
}

impl Members {
    /// Members of `value`, which must be an object.
    fn object(value: Value, at: &'static str) -> Result<Self, String> {
        match value {
            Value::Object(map) => Ok(Self { map, at }),
            other => Err(format!("{at}expected an object, got {other}")),
        }
    }

    /// Whether member `name` is present.
    fn has(&self, name: &str) -> bool {
        self.map.contains_key(name)
    }

    /// Decode optional member `name`; `null` counts as absent.
    fn opt<T: DeserializeOwned>(&mut self, name: &str) -> Result<Option<T>, String> {
        self.take::<Option<T>>(name).map(Option::flatten)
    }

    /// Decode required member `name`.
    fn req<T: DeserializeOwned>(&mut self, name: &str) -> Result<T, String> {
        self.take(name)?
            .ok_or_else(|| format!("{}missing {name:?}", self.at))
    }

    /// Remove and decode member `name` when present.
    fn take<T: DeserializeOwned>(&mut self, name: &str) -> Result<Option<T>, String> {
        let at = self.at;
        self.map
            .remove(name)
            .map(|value| T::deserialize(value).map_err(|err| format!("{at}{name:?}: {err}")))
            .transpose()
    }

    /// Decode the `{const, title}` option list `name`.
    fn options(&mut self, name: &str) -> Result<Vec<EnumOption>, String> {
        let at = self.at;
        self.req::<Vec<Value>>(name)?
            .into_iter()
            .enumerate()
            .map(|(index, option)| {
                Self::option(option).map_err(|err| format!("{at}{name:?} item {index}: {err}"))
            })
            .collect()
    }

    /// Decode one `{const, title}` option.
    fn option(value: Value) -> Result<EnumOption, String> {
        let mut option = Self::object(value, "")?;
        Ok(EnumOption {
            const_value: option.req("const")?,
            title: option.req("title")?,
        })
    }

    /// Decode an `array` schema by its `items.anyOf`, else its `items.enum`.
    fn multi_select(
        &mut self,
        title: Option<Cow<'static, str>>,
        description: Option<Cow<'static, str>>,
    ) -> Result<EnumSchema, String> {
        let mut items = Self::object(self.req("items")?, "\"items\": ")?;
        let titled = items.has("anyOf");
        if !titled && !items.has("enum") {
            return Err("\"items\": missing \"anyOf\" or \"enum\"".to_owned());
        }
        let min_items = self.opt("minItems")?;
        let max_items = self.opt("maxItems")?;
        let default = self.opt("default")?;
        Ok(if titled {
            EnumSchema::TitledMultiSelect(TitledMultiSelectEnumSchema {
                type_: ArrayTypeConst,
                items: TitledEnumItems {
                    any_of: items.options("anyOf")?,
                },
                min_items,
                max_items,
                title,
                description,
                default,
            })
        } else {
            EnumSchema::UntitledMultiSelect(UntitledMultiSelectEnumSchema {
                type_: ArrayTypeConst,
                items: UntitledEnumItems {
                    type_: items.req("type")?,
                    enum_values: items.req("enum")?,
                },
                min_items,
                max_items,
                title,
                description,
                default,
            })
        })
    }
}
