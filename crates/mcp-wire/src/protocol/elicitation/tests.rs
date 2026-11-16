use super::*;
use serde_json::{Value, json};

use crate::protocol::types::{
    CreateElicitationRequest, CreateElicitationResult, ElicitationAction,
};
use crate::protocol::version::ProtocolVersion::{V2025_06_18, V2025_11_25};
use crate::protocol::version::UnsupportedByVersion;
use std::assert_matches;
use std::borrow::Cow;

#[test]
fn test_untitled_single_select_serialization() {
    let schema = UntitledSingleSelectEnumSchema::new(vec![
        "red".to_string(),
        "green".to_string(),
        "blue".to_string(),
    ])
    .title("Color")
    .with_default("red");

    let json = serde_json::to_value(&schema).unwrap();
    assert_eq!(json["type"], "string");
    assert_eq!(json["enum"], json!(["red", "green", "blue"]));
    assert_eq!(json["title"], "Color");
    assert_eq!(json["default"], "red");
}

#[test]
fn test_titled_single_select_serialization() {
    let schema = TitledSingleSelectEnumSchema::from_pairs([
        ("#FF0000", "Red"),
        ("#00FF00", "Green"),
        ("#0000FF", "Blue"),
    ])
    .title("Color Selection");

    let json = serde_json::to_value(&schema).unwrap();
    assert_eq!(json["type"], "string");
    assert_eq!(json["oneOf"][0]["const"], "#FF0000");
    assert_eq!(json["oneOf"][0]["title"], "Red");
    assert_eq!(json["title"], "Color Selection");
}

#[test]
fn test_untitled_multi_select_serialization() {
    let schema = UntitledMultiSelectEnumSchema::new(vec![
        "apple".to_string(),
        "banana".to_string(),
        "cherry".to_string(),
    ])
    .min_items(1)
    .max_items(2)
    .title("Fruits");

    let json = serde_json::to_value(&schema).unwrap();
    assert_eq!(json["type"], "array");
    assert_eq!(json["items"]["type"], "string");
    assert_eq!(json["items"]["enum"], json!(["apple", "banana", "cherry"]));
    assert_eq!(json["minItems"], 1);
    assert_eq!(json["maxItems"], 2);
}

#[test]
fn test_titled_multi_select_serialization() {
    let schema =
        TitledMultiSelectEnumSchema::from_pairs([("opt1", "Option 1"), ("opt2", "Option 2")])
            .items_range(1, 2)
            .with_default(vec!["opt1".to_string()]);

    let json = serde_json::to_value(&schema).unwrap();
    assert_eq!(json["type"], "array");
    assert_eq!(json["items"]["anyOf"][0]["const"], "opt1");
    assert_eq!(json["items"]["anyOf"][0]["title"], "Option 1");
    assert_eq!(json["default"], json!(["opt1"]));
}

#[test]
fn test_legacy_enum_serialization() {
    let schema = LegacyTitledEnumSchema::from_pairs([("v1", "Value 1"), ("v2", "Value 2")]);

    let json = serde_json::to_value(&schema).unwrap();
    assert_eq!(json["type"], "string");
    assert_eq!(json["enum"], json!(["v1", "v2"]));
    assert_eq!(json["enumNames"], json!(["Value 1", "Value 2"]));
}

/// Inverted bounds and mismatched enum names are errors, never panics.
#[test]
fn inverted_bounds_are_errors() {
    let bounds: [(&str, Result<(), &str>); 6] = [
        ("number", NumberSchema::new().range(2.0, 1.0).map(drop)),
        ("integer", IntegerSchema::new().range(2, 1).map(drop)),
        ("length", StringSchema::new().length(2, 1).map(drop)),
        (
            "required number",
            ElicitationSchema::builder()
                .required_number("n", 2.0, 1.0)
                .map(drop),
        ),
        (
            "required integer",
            ElicitationSchema::builder()
                .required_integer("n", 2, 1)
                .map(drop),
        ),
        (
            "legacy names",
            LegacyTitledEnumSchema::new(vec!["a".into()], Vec::new()).map(drop),
        ),
    ];
    for (case, result) in bounds {
        assert!(result.is_err(), "{case}");
    }
    assert!(IntegerSchema::new().range(1, 1).is_ok());
    assert!(StringSchema::new().length(1, 1).is_ok());
}

#[test]
fn test_enum_schema_factory_methods() {
    let single = EnumSchema::single_select(["a", "b", "c"]);
    assert_matches!(single, EnumSchema::UntitledSingleSelect(_));

    let titled = EnumSchema::single_select_titled([("val", "Title")]);
    assert_matches!(titled, EnumSchema::TitledSingleSelect(_));

    let multi = EnumSchema::multi_select(["x", "y"]);
    assert_matches!(multi, EnumSchema::UntitledMultiSelect(_));

    let multi_titled = EnumSchema::multi_select_titled([("a", "A"), ("b", "B")]);
    assert_matches!(multi_titled, EnumSchema::TitledMultiSelect(_));
}

/// Variant name, for table assertions.
const fn variant(schema: &PrimitiveSchema) -> &'static str {
    match schema {
        PrimitiveSchema::Enum(EnumSchema::TitledSingleSelect(_)) => "TitledSingleSelect",
        PrimitiveSchema::Enum(EnumSchema::TitledMultiSelect(_)) => "TitledMultiSelect",
        PrimitiveSchema::Enum(EnumSchema::UntitledMultiSelect(_)) => "UntitledMultiSelect",
        PrimitiveSchema::Enum(EnumSchema::UntitledSingleSelect(_)) => "UntitledSingleSelect",
        PrimitiveSchema::Enum(EnumSchema::Legacy(_)) => "Legacy",
        PrimitiveSchema::String(_) => "String",
        PrimitiveSchema::Number(_) => "Number",
        PrimitiveSchema::Integer(_) => "Integer",
        PrimitiveSchema::Boolean(_) => "Boolean",
    }
}

/// (variant, property document, re-encoding when members are dropped; `None` = the document)
const DECODED: &[(&str, &str, Option<&str>)] = &[
    // 2025-11-25 client/elicitation.mdx, "Requested Schema"
    (
        "String",
        r#"{"type": "string", "title": "Display Name", "description": "Description text", "minLength": 3, "maxLength": 50, "pattern": "^[A-Za-z]+$", "format": "email", "default": "user@example.com"}"#,
        None,
    ),
    (
        "Number",
        r#"{"type": "number", "title": "Display Name", "description": "Description text", "minimum": 0, "maximum": 100, "default": 50}"#,
        None,
    ),
    (
        "Integer",
        r#"{"type": "integer", "title": "Display Name", "description": "Description text", "minimum": 0, "maximum": 100, "default": 50}"#,
        None,
    ),
    (
        "Boolean",
        r#"{"type": "boolean", "title": "Display Name", "description": "Description text", "default": false}"#,
        None,
    ),
    (
        "UntitledSingleSelect",
        r#"{"type": "string", "title": "Color Selection", "description": "Choose your favorite color", "enum": ["Red", "Green", "Blue"], "default": "Red"}"#,
        None,
    ),
    (
        "TitledSingleSelect",
        r##"{"type": "string", "title": "Color Selection", "description": "Choose your favorite color", "oneOf": [{"const": "#FF0000", "title": "Red"}, {"const": "#00FF00", "title": "Green"}, {"const": "#0000FF", "title": "Blue"}], "default": "#FF0000"}"##,
        None,
    ),
    (
        "UntitledMultiSelect",
        r#"{"type": "array", "title": "Color Selection", "description": "Choose your favorite colors", "minItems": 1, "maxItems": 2, "items": {"type": "string", "enum": ["Red", "Green", "Blue"]}, "default": ["Red", "Green"]}"#,
        None,
    ),
    (
        "TitledMultiSelect",
        r##"{"type": "array", "title": "Color Selection", "description": "Choose your favorite colors", "minItems": 1, "maxItems": 2, "items": {"anyOf": [{"const": "#FF0000", "title": "Red"}, {"const": "#00FF00", "title": "Green"}, {"const": "#0000FF", "title": "Blue"}]}, "default": ["#FF0000", "#00FF00"]}"##,
        None,
    ),
    // 2025-06-18 client/elicitation.mdx: EnumSchema with enumNames; `default` only on booleans
    (
        "Legacy",
        r#"{"type": "string", "title": "Display Name", "description": "Description text", "enum": ["option1", "option2", "option3"], "enumNames": ["Option 1", "Option 2", "Option 3"]}"#,
        None,
    ),
    (
        "String",
        r#"{"type": "string", "title": "Display Name", "description": "Description text", "minLength": 3, "maxLength": 50, "format": "email"}"#,
        None,
    ),
    (
        "Number",
        r#"{"type": "number", "title": "Display Name", "description": "Description text", "minimum": 0, "maximum": 100}"#,
        None,
    ),
    // NumberSchema bounds are spec `number`s, also for "integer"
    (
        "Integer",
        r#"{"type": "integer", "minimum": 0.5, "maximum": 100.0}"#,
        None,
    ),
    // overlapping shapes pick the most specific variant
    (
        "UntitledSingleSelect",
        r#"{"type": "string", "enum": ["a", "b"]}"#,
        None,
    ),
    (
        "Legacy",
        r#"{"type": "string", "enum": ["a", "b"], "enumNames": ["A", "B"]}"#,
        None,
    ),
    (
        "TitledSingleSelect",
        r#"{"type": "string", "oneOf": [{"const": "a", "title": "A"}], "enum": ["a"], "enumNames": ["A"]}"#,
        Some(r#"{"type": "string", "oneOf": [{"const": "a", "title": "A"}]}"#),
    ),
    (
        "TitledMultiSelect",
        r#"{"type": "array", "items": {"type": "string", "enum": ["a"], "anyOf": [{"const": "a", "title": "A"}]}}"#,
        Some(r#"{"type": "array", "items": {"anyOf": [{"const": "a", "title": "A"}]}}"#),
    ),
    (
        "String",
        r#"{"type": "string", "enumNames": ["A"]}"#,
        Some(r#"{"type": "string"}"#),
    ),
    // unknown members are ignored, `null` optional members are absent
    (
        "Boolean",
        r#"{"type": "boolean", "x-widget": "toggle", "title": null}"#,
        Some(r#"{"type": "boolean"}"#),
    ),
];

#[test]
fn property_schemas_decode_by_type_then_most_specific_member() {
    for (expected, document, reencoded) in DECODED {
        let schema: PrimitiveSchema = serde_json::from_str(document).unwrap();
        assert_eq!(variant(&schema), *expected, "{document}");
        assert_eq!(
            serde_json::to_value(&schema).unwrap(),
            serde_json::from_str::<Value>(reencoded.unwrap_or(document)).unwrap(),
            "{document}"
        );
    }

    // 2025-11-25 "Example: Structured Data Request"
    let requested = json!({
        "type": "object",
        "properties": {
            "name": { "type": "string", "description": "Your full name" },
            "email": { "type": "string", "format": "email", "description": "Your email address" },
            "age": { "type": "number", "minimum": 18, "description": "Your age" }
        },
        "required": ["name", "email"]
    });
    let schema = parse_elicitation_schema(&requested).unwrap();
    assert_eq!(serde_json::to_value(&schema).unwrap(), requested);
}

/// (property document, its 2025-06-18 rendering; `None` = refused): every
/// variant, from the 2025-11-25 client/elicitation.mdx examples.
const RENDERED_2025_06_18: &[(&str, Option<&str>)] = &[
    // StringSchema: no `pattern` or `default` (2025-06-18 schema.ts:1491-1498)
    (
        r#"{"type": "string", "title": "Display Name", "description": "Description text", "minLength": 3, "maxLength": 50, "pattern": "^[A-Za-z]+$", "format": "email", "default": "user@example.com"}"#,
        Some(
            r#"{"type": "string", "title": "Display Name", "description": "Description text", "minLength": 3, "maxLength": 50, "format": "email"}"#,
        ),
    ),
    // ...and the same formats (:1497)
    (
        r#"{"type": "string", "format": "uri"}"#,
        Some(r#"{"type": "string", "format": "uri"}"#),
    ),
    (
        r#"{"type": "string", "format": "date"}"#,
        Some(r#"{"type": "string", "format": "date"}"#),
    ),
    (
        r#"{"type": "string", "format": "date-time"}"#,
        Some(r#"{"type": "string", "format": "date-time"}"#),
    ),
    // NumberSchema: no `default` (:1503-1515)
    (
        r#"{"type": "number", "title": "Display Name", "description": "Description text", "minimum": 0, "maximum": 100, "default": 50}"#,
        Some(
            r#"{"type": "number", "title": "Display Name", "description": "Description text", "minimum": 0, "maximum": 100}"#,
        ),
    ),
    (
        r#"{"type": "integer", "minimum": 0, "maximum": 100, "default": 50}"#,
        Some(r#"{"type": "integer", "minimum": 0, "maximum": 100}"#),
    ),
    // BooleanSchema keeps its `default` (:1524)
    (
        r#"{"type": "boolean", "title": "Display Name", "description": "Description text", "default": false}"#,
        Some(
            r#"{"type": "boolean", "title": "Display Name", "description": "Description text", "default": false}"#,
        ),
    ),
    // EnumSchema: `enum`, titles as `enumNames`, no `default` (:1530-1536)
    (
        r#"{"type": "string", "title": "Color Selection", "description": "Choose your favorite color", "enum": ["Red", "Green", "Blue"], "default": "Red"}"#,
        Some(
            r#"{"type": "string", "title": "Color Selection", "description": "Choose your favorite color", "enum": ["Red", "Green", "Blue"]}"#,
        ),
    ),
    (
        r##"{"type": "string", "title": "Color Selection", "description": "Choose your favorite color", "oneOf": [{"const": "#FF0000", "title": "Red"}, {"const": "#00FF00", "title": "Green"}, {"const": "#0000FF", "title": "Blue"}], "default": "#FF0000"}"##,
        Some(
            r##"{"type": "string", "title": "Color Selection", "description": "Choose your favorite color", "enum": ["#FF0000", "#00FF00", "#0000FF"], "enumNames": ["Red", "Green", "Blue"]}"##,
        ),
    ),
    (
        r#"{"type": "string", "enum": ["a", "b"], "enumNames": ["A", "B"], "default": "a"}"#,
        Some(r#"{"type": "string", "enum": ["a", "b"], "enumNames": ["A", "B"]}"#),
    ),
    // no multi-select (:1485-1486)
    (
        r#"{"type": "array", "minItems": 1, "items": {"type": "string", "enum": ["Red", "Green"]}, "default": ["Red"]}"#,
        None,
    ),
    (
        r##"{"type": "array", "items": {"anyOf": [{"const": "#FF0000", "title": "Red"}]}}"##,
        None,
    ),
];

#[test]
fn property_schemas_render_per_version() {
    let form = |property: &str| {
        let property: Value = serde_json::from_str(property).unwrap();
        json!({"type": "object", "properties": {"field": property}})
    };
    for (document, expected) in RENDERED_2025_06_18 {
        let schema = parse_elicitation_schema(&form(document)).unwrap();
        // 2025-11-25 takes the schema itself, so its bytes are unchanged.
        assert_matches!(
            schema.for_version(V2025_11_25),
            Ok(Cow::Borrowed(latest)) if std::ptr::eq(latest, &raw const schema),
            "{document}"
        );
        let rendered = schema
            .for_version(V2025_06_18)
            .map(|rendered| serde_json::to_value(&*rendered).unwrap());
        let refused = UnsupportedByVersion {
            feature: r#"multi-select enum property "field""#.into(),
            version: V2025_06_18,
        };
        assert_eq!(rendered, expected.map(form).ok_or(refused), "{document}");
    }

    // The form keeps `type`, `properties` and `required` (2025-06-18 schema.ts:1469-1475).
    let schema = ElicitationSchema::builder()
        .schema_uri("https://json-schema.org/draft/2020-12/schema")
        .title("Order")
        .description("Your order")
        .required_enum_titled("size", [("s", "Small"), ("l", "Large")])
        .build()
        .unwrap();
    assert_eq!(
        serde_json::to_value(&*schema.for_version(V2025_06_18).unwrap()).unwrap(),
        json!({
            "type": "object",
            "properties": {
                "size": {"type": "string", "enum": ["s", "l"], "enumNames": ["Small", "Large"]}
            },
            "required": ["size"]
        })
    );
    let refused = ElicitationSchema::builder()
        .required_multi_enum("tags", ["a"])
        .build()
        .unwrap()
        .for_version(V2025_06_18)
        .unwrap_err();
    assert_eq!(
        refused.to_string(),
        r#"multi-select enum property "tags" is not defined in protocol version 2025-06-18"#
    );
}

#[test]
fn content_values_follow_the_negotiated_version() {
    // (undeclared field value, accepted under 2025-11-25, under 2025-06-18):
    // a string array is 2025-11-25 only (schema.ts:2485; 2025-06-18:1556).
    let cases = [
        (json!("text"), true, true),
        (json!(1.5), true, true),
        (json!(false), true, true),
        (json!(["a"]), true, false),
        (json!([1]), false, false),
        (json!(null), false, false),
        (json!({}), false, false),
    ];
    let schema = ElicitationSchema::new(std::collections::BTreeMap::new());
    for (value, latest, earlier) in cases {
        for (version, accepted) in [(V2025_11_25, latest), (V2025_06_18, earlier)] {
            let result = schema.validate_content(&json!({ "extra": value }), version);
            assert_eq!(
                result.is_ok(),
                accepted,
                "{value} under {version}: {result:?}"
            );
        }
    }
}

#[test]
fn property_schema_errors_name_the_property_and_the_problem() {
    let cases = [
        (json!({ "title": "Color" }), r#"missing "type""#),
        (json!({ "type": "date" }), r#"unknown "type" "date""#),
        (
            json!({ "type": ["string", "null"] }),
            r#"unknown "type" ["string","null"]"#,
        ),
        (
            json!({ "type": "object", "properties": {} }),
            r#"unknown "type" "object""#,
        ),
        (json!("string"), r#"expected an object, got "string""#),
        (json!({ "type": "array" }), r#"missing "items""#),
        (
            json!({ "type": "array", "items": "string" }),
            r#""items": expected an object, got "string""#,
        ),
        (
            json!({ "type": "array", "items": { "type": "string" } }),
            r#""items": missing "anyOf" or "enum""#,
        ),
        (
            json!({ "type": "array", "items": { "enum": ["a"] } }),
            r#""items": missing "type""#,
        ),
        (
            json!({ "type": "string", "oneOf": [
                { "const": "r", "title": "Red" },
                { "const": "g", "title": "Green" },
                { "title": "Blue" }
            ] }),
            r#""oneOf" item 2: missing "const""#,
        ),
        (
            json!({ "type": "string", "oneOf": [{ "const": "r" }] }),
            r#""oneOf" item 0: missing "title""#,
        ),
        (
            json!({ "type": "array", "items": { "anyOf": [
                { "const": "r", "title": "Red" },
                { "const": 1, "title": "One" }
            ] } }),
            r#""items": "anyOf" item 1: "const": invalid type: integer `1`, expected a string"#,
        ),
        (
            json!({ "type": "string", "enum": ["a"], "enumNames": null }),
            r#""enumNames": invalid type: null, expected a sequence"#,
        ),
        (
            json!({ "type": "string", "minLength": "3" }),
            r#""minLength": invalid type: string "3", expected u32"#,
        ),
        (
            json!({ "type": "string", "format": "phone" }),
            "\"format\": unknown variant `phone`, expected one of `email`, `uri`, `date`, `date-time`",
        ),
    ];
    for (property, expected) in cases {
        assert_eq!(
            serde_json::from_value::<PrimitiveSchema>(property.clone())
                .unwrap_err()
                .to_string(),
            expected,
        );
        assert_eq!(
            parse_elicitation_schema(&json!({
                "type": "object", "properties": { "color": property }
            })),
            Err(ElicitationValidationError::InvalidSchema(format!(
                "requestedSchema is not a supported form schema: property \"color\": {expected}"
            ))),
        );
    }

    assert_eq!(
        serde_json::from_value::<EnumSchema>(json!({ "type": "number" }))
            .unwrap_err()
            .to_string(),
        r#"not an enum schema: no "oneOf", "enum" or array "items""#,
    );
}

#[test]
fn test_elicitation_builder_with_enum() {
    let schema = ElicitationSchema::builder()
        .required_enum("color", ["red", "green", "blue"])
        .required_enum_titled("size", [("s", "Small"), ("m", "Medium"), ("l", "Large")])
        .required_multi_enum("tags", ["tag1", "tag2", "tag3"])
        .build()
        .unwrap();

    assert_eq!(
        schema.required,
        Some(vec![
            "color".to_string(),
            "size".to_string(),
            "tags".to_string()
        ])
    );
    assert!(schema.properties.contains_key("color"));
    assert!(schema.properties.contains_key("size"));
    assert!(schema.properties.contains_key("tags"));
}

#[test]
fn test_enum_schema_roundtrip() {
    let original = EnumSchema::single_select_titled([("#FF0000", "Red"), ("#00FF00", "Green")])
        .title("Pick a color")
        .description("Choose your favorite");

    let json = serde_json::to_string(&original).unwrap();
    let parsed: EnumSchema = serde_json::from_str(&json).unwrap();

    let json2 = serde_json::to_string(&parsed).unwrap();
    assert_eq!(json, json2);
}

#[test]
fn test_primitive_schema_with_enum() {
    let enum_schema = EnumSchema::single_select(["a", "b", "c"]);
    let primitive = PrimitiveSchema::Enum(enum_schema);

    let json = serde_json::to_value(&primitive).unwrap();
    assert_eq!(json["type"], "string");
    assert_eq!(json["enum"], json!(["a", "b", "c"]));
}

#[test]
fn test_schema_serializes_spec_metadata() {
    let schema = ElicitationSchema::builder()
        .schema_uri("https://json-schema.org/draft/2020-12/schema")
        .title("Profile")
        .required_property(
            "name",
            PrimitiveSchema::String(
                StringSchema::new()
                    .pattern("^[A-Z][a-z]+$")
                    .with_default("Ada"),
            ),
        )
        .property(
            "score",
            PrimitiveSchema::Number(NumberSchema::new().with_default(0.5)),
        )
        .property(
            "age",
            PrimitiveSchema::Integer(IntegerSchema::new().with_default(42)),
        )
        .build()
        .unwrap();

    let json = serde_json::to_value(&schema).unwrap();
    assert_eq!(
        json["$schema"],
        "https://json-schema.org/draft/2020-12/schema"
    );
    assert_eq!(json["title"], "Profile");
    assert_eq!(json["properties"]["name"]["pattern"], "^[A-Z][a-z]+$");
    assert_eq!(json["properties"]["name"]["default"], "Ada");
    assert_eq!(json["properties"]["score"]["default"], 0.5);
    assert_eq!(json["properties"]["age"]["default"], 42);
}

#[test]
fn test_confirm_request_builder_validates_response() {
    let request = CreateElicitationRequest::confirm("Continue?");
    let result = CreateElicitationResult {
        action: ElicitationAction::Accept,
        content: Some(json!({ "confirmed": true })),
    };

    request.validate_result(&result, V2025_11_25).unwrap();
}

#[test]
fn test_elicitation_result_validation_rejects_missing_required_field() {
    let schema = ElicitationSchema::builder()
        .required_email("email")
        .build()
        .unwrap();
    let result = CreateElicitationResult {
        action: ElicitationAction::Accept,
        content: Some(json!({})),
    };

    assert_matches!(
        schema.validate_result(&result, V2025_11_25),
        Err(ElicitationValidationError::MissingRequired { field }) if field == "email"
    );
}

#[test]
fn test_elicitation_result_validation_rejects_invalid_values() {
    let schema = ElicitationSchema::builder()
        .required_property(
            "name",
            PrimitiveSchema::String(
                StringSchema::new()
                    .length(2, 8)
                    .unwrap()
                    .pattern("^[A-Z]+$"),
            ),
        )
        .required_property(
            "age",
            PrimitiveSchema::Integer(IntegerSchema::new().range(18, 130).unwrap()),
        )
        .required_enum("color", ["red", "green"])
        .required_multi_enum("tags", ["rust", "mcp"])
        .build()
        .unwrap();

    for content in [
        json!({ "name": "ada", "age": 42, "color": "red", "tags": ["rust"] }),
        json!({ "name": "ADA", "age": 17, "color": "red", "tags": ["rust"] }),
        json!({ "name": "ADA", "age": 42, "color": "blue", "tags": ["rust"] }),
        json!({ "name": "ADA", "age": 42, "color": "red", "tags": ["go"] }),
    ] {
        let result = CreateElicitationResult {
            action: ElicitationAction::Accept,
            content: Some(content),
        };
        assert_matches!(
            schema.validate_result(&result, V2025_11_25),
            Err(ElicitationValidationError::InvalidField { .. })
        );
    }
}

#[test]
fn test_elicitation_result_validation_checks_formats_and_actions() {
    let schema = ElicitationSchema::builder()
        .required_email("email")
        .required_property("site", PrimitiveSchema::String(StringSchema::uri()))
        .required_property("birthday", PrimitiveSchema::String(StringSchema::date()))
        .required_property(
            "created",
            PrimitiveSchema::String(StringSchema::date_time()),
        )
        .build()
        .unwrap();

    schema
        .validate_result(
            &CreateElicitationResult {
                action: ElicitationAction::Accept,
                content: Some(json!({
                    "email": "ada@example.com",
                    "site": "https://example.com",
                    "birthday": "1815-12-10",
                    "created": "2026-05-09T12:00:00Z"
                })),
            },
            V2025_11_25,
        )
        .unwrap();

    assert_matches!(
        schema.validate_result(
            &CreateElicitationResult {
                action: ElicitationAction::Accept,
                content: Some(json!({
                    "email": "not-an-email",
                    "site": "https://example.com",
                    "birthday": "1815-12-10",
                    "created": "2026-05-09T12:00:00Z"
                })),
            },
            V2025_11_25
        ),
        Err(ElicitationValidationError::InvalidField { .. })
    );

    assert_matches!(
        schema.validate_result(
            &CreateElicitationResult {
                action: ElicitationAction::Decline,
                content: Some(json!({ "email": "ada@example.com" })),
            },
            V2025_11_25
        ),
        Err(ElicitationValidationError::NonAcceptWithContent)
    );
}

/// (property schema, accepted values, rejected values)
const VALUES: &[(&str, &str, &str)] = &[
    // number: any JSON number within the inclusive bounds
    (
        r#"{"type": "number", "minimum": 0, "maximum": 1.5}"#,
        "[0, 1, 1.5]",
        r#"[-0.1, 1.6, "1", null]"#,
    ),
    // integer: zero fractional part, as in JSON Schema; bounds are spec numbers
    (
        r#"{"type": "integer", "minimum": 0.5, "maximum": 10}"#,
        "[1, 10, 10.0]",
        r#"[0, 2.5, 11, "1"]"#,
    ),
    // integers compare exactly past 2^53
    (
        r#"{"type": "integer", "maximum": 9007199254740993}"#,
        "[9007199254740993]",
        "[9007199254740994]",
    ),
    (r#"{"type": "boolean"}"#, "[true, false]", r#"["true", 1]"#),
    // lengths count characters
    (
        r#"{"type": "string", "minLength": 2, "maxLength": 3}"#,
        r#"["ña", "ñañ"]"#,
        r#"["ñ", "ñaña", 3]"#,
    ),
    (
        r#"{"type": "string", "pattern": "^[A-Z]+$"}"#,
        r#"["ADA"]"#,
        r#"["ada", "ADA1"]"#,
    ),
    (
        r#"{"type": "string", "format": "email"}"#,
        r#"["ada@example.com"]"#,
        r#"["not-an-email"]"#,
    ),
    // RFC 3986 URI
    (
        r#"{"type": "string", "format": "uri"}"#,
        r#"["https://example.com/a?b#c", "urn:isbn:0451450523"]"#,
        r#"["1http:x", "a:%zz", "http://[::1", "https://example.com/ä", "/relative"]"#,
    ),
    // RFC 3339 full-date
    (
        r#"{"type": "string", "format": "date"}"#,
        r#"["1815-12-10", "2024-02-29"]"#,
        r#"["1815-12-1", "2024-02-30", "18151210", "1815-12-10T00:00:00Z"]"#,
    ),
    // RFC 3339 date-time; `T` and `Z` in either case
    (
        r#"{"type": "string", "format": "date-time"}"#,
        r#"["2026-05-09T12:00:00Z", "2026-05-09t12:00:00.5z", "2026-05-09T12:00:00.123+05:30", "2026-05-09T12:00:00-00:00", "2016-12-31T23:59:60Z"]"#,
        r#"["2026-05-09T12:00Z", "2026-05-09T12Z", "2026-05-09 12:00:00Z", "2026-05-09T12:00:00", "2026-05-09T12:00:00+05", "2026-05-09T12:00:00+0530", "2026-05-09T12:00:00.Z", "2026-05-09T12:00:00,5Z", "20260509T120000Z", "2026-05-09T12:00:00Z[UTC]", "2026-05-09T24:00:00Z"]"#,
    ),
    // single select: option values, not titles
    (
        r##"{"type": "string", "oneOf": [{"const": "#FF0000", "title": "Red"}]}"##,
        r##"["#FF0000"]"##,
        r##"["Red", ["#FF0000"]]"##,
    ),
    (
        r#"{"type": "string", "enum": ["a"], "enumNames": ["A"]}"#,
        r#"["a"]"#,
        r#"["A"]"#,
    ),
    // multi select: a string[] within the count bounds and options
    (
        r#"{"type": "array", "minItems": 1, "maxItems": 2, "items": {"type": "string", "enum": ["a", "b", "c"]}}"#,
        r#"[["a"], ["a", "b"]]"#,
        r#"[[], ["a", "b", "c"], ["d"], [1], "a"]"#,
    ),
    (
        r##"{"type": "array", "items": {"anyOf": [{"const": "#FF0000", "title": "Red"}]}}"##,
        r##"[["#FF0000"], []]"##,
        r#"[["Red"]]"#,
    ),
];

#[test]
fn property_values_follow_spec_types_bounds_formats_and_options() {
    for (property, accepted, rejected) in VALUES {
        let property: Value = serde_json::from_str(property).unwrap();
        let schema = parse_elicitation_schema(&json!({
            "type": "object", "properties": { "field": property }
        }))
        .unwrap();
        for (values, valid) in [(accepted, true), (rejected, false)] {
            for value in serde_json::from_str::<Vec<Value>>(values).unwrap() {
                let result = schema.validate_content(&json!({ "field": value }), V2025_11_25);
                assert_eq!(result.is_ok(), valid, "{property} with {value}: {result:?}");
            }
        }
    }
}

#[test]
fn string_pattern_compiles_once_and_stays_out_of_equality_and_wire() {
    let fresh = PrimitiveSchema::String(StringSchema::new().pattern("^[A-Z]+$"));
    let used = fresh.clone();
    used.validate_value("name", &json!("ADA")).unwrap();
    let PrimitiveSchema::String(StringSchema {
        pattern: Some(pattern),
        ..
    }) = &used
    else {
        panic!("built as a string schema with a pattern");
    };
    assert!(std::ptr::eq(
        pattern.regex().unwrap(),
        pattern.regex().unwrap()
    ));
    assert_eq!(used, fresh);
    assert_eq!(
        serde_json::to_value(&used).unwrap(),
        serde_json::to_value(&fresh).unwrap()
    );

    let invalid = PrimitiveSchema::String(StringSchema::new().pattern("("));
    for _ in 0..2 {
        assert_matches!(
            invalid.validate_value("name", &json!("x")),
            Err(ElicitationValidationError::InvalidSchema(_))
        );
    }
}
