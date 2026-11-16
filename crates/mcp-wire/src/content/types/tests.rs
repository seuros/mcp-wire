use super::*;
use crate::test_support::priority;

fn meta() -> Map<String, Value> {
    serde_json::from_value(serde_json::json!({"traceId": "trace-7"})).unwrap()
}

fn wire(content: &impl Serialize) -> Value {
    serde_json::to_value(content).unwrap()
}

#[test]
fn test_text_content() {
    let content = TextContent::new("Hello, world!");
    assert_eq!(wire(&content)["type"], "text");
    assert_eq!(content.text, "Hello, world!");
    assert!(content.annotations.is_none());
}

#[test]
fn test_text_with_annotations() {
    let ann = Annotations::new().with_priority(priority(0.9));
    let content = TextContent::new("Important!").with_annotations(ann.clone());

    assert_eq!(content.annotations, Some(ann));
}

#[test]
fn test_image_content() {
    let content = ImageContent::new("base64data", "image/png");
    assert_eq!(wire(&content)["type"], "image");
    assert_eq!(content.data, "base64data");
    std::assert_matches!(content.mime_type, Cow::Borrowed("image/png"));
}

#[test]
fn test_resource_link() {
    let link = ResourceLink::new("file://test.txt")
        .with_name("test-file")
        .with_title("Test File")
        .with_description("A test file")
        .with_mime_type("text/plain")
        .with_icons(vec![
            Icon::new("data:image/png;base64,aWNvbg==")
                .unwrap()
                .with_mime_type("image/png")
                .with_sizes([IconSize::Pixels {
                    width: 48,
                    height: 48,
                }])
                .with_theme(IconTheme::Dark),
        ])
        .with_annotations(Annotations::new().with_priority(priority(0.8)))
        .with_size(9)
        .with_meta(meta());

    assert_eq!(link.uri, "file://test.txt");
    assert_eq!(link.name, "test-file");
    assert_eq!(link.title, Some("Test File".to_string()));
    let value = wire(&link);
    assert_eq!(value["type"], "resource_link");
    assert_eq!(value["description"], "A test file");
    assert_eq!(value["mimeType"], "text/plain");
    assert_eq!(value["icons"][0]["sizes"], serde_json::json!(["48x48"]));
    assert_eq!(value["icons"][0]["theme"], "dark");
    assert_eq!(value["annotations"]["priority"], 0.8);
    assert_eq!(value["size"], 9);
    assert_eq!(value["_meta"]["traceId"], "trace-7");

    assert_eq!(serde_json::from_value::<ResourceLink>(value).unwrap(), link);
}

#[test]
fn test_embedded_resource() {
    let resource = ResourceContent::text("file://test.txt", "Hello");
    let content = EmbeddedResource::new(resource);
    let json = wire(&content);

    assert_eq!(json["type"], "resource");
    assert_eq!(json["resource"]["uri"], "file://test.txt");
    assert_eq!(json["resource"]["text"], "Hello");
}

#[test]
fn test_serialization() {
    let content = TextContent::new("test");
    let json = wire(&content);

    assert_eq!(json["text"], "test");
    assert_eq!(json["type"], "text");
}

#[test]
fn test_annotations_are_nested() {
    let content = TextContent::new("test")
        .with_annotations(Annotations::new().add_audience(crate::protocol::types::Role::User));
    let json = wire(&content);

    assert_eq!(json["annotations"]["audience"], serde_json::json!(["user"]));
    assert!(json.get("audience").is_none());

    let decoded: TextContent = serde_json::from_value(json).unwrap();
    assert_eq!(
        decoded.annotations.unwrap().audience,
        Some(vec![crate::protocol::types::Role::User])
    );
}

#[test]
fn test_invalid_annotation_audience_is_rejected() {
    let json = serde_json::json!({
        "type": "text",
        "text": "test",
        "annotations": {"audience": ["admin"]}
    });

    assert!(serde_json::from_value::<TextContent>(json).is_err());
}

#[test]
fn content_block_preserves_annotations_and_meta_for_every_variant() {
    let annotations = Annotations::new()
        .with_audience(vec![crate::protocol::types::Role::Assistant])
        .with_priority(priority(0.6));
    let blocks = vec![
        ContentBlock::text("text"),
        ContentBlock::image("aW1hZ2U=", "image/png"),
        ContentBlock::audio("YXVkaW8=", "audio/wav"),
        ContentBlock::resource_named("guide", "file:///guide.md"),
        ContentBlock::embedded_resource(
            ResourceContent::text("file:///guide.md", "guide").with_meta(meta()),
        ),
    ];

    for block in blocks {
        let block = block
            .with_annotations(annotations.clone())
            .with_meta(meta());
        let value = serde_json::to_value(&block).unwrap();
        assert_eq!(
            value["annotations"]["audience"],
            serde_json::json!(["assistant"])
        );
        assert_eq!(value["annotations"]["priority"], 0.6);
        assert_eq!(value["_meta"]["traceId"], "trace-7");
        assert_eq!(
            serde_json::from_value::<ContentBlock>(value).unwrap(),
            block
        );
    }
}

#[test]
fn content_block_rejects_unknown_or_mismatched_types() {
    use crate::test_support::decodes as accepts;
    type Accepts = fn(&Value) -> bool;

    assert!(
        serde_json::from_value::<ContentBlock>(serde_json::json!({
            "type": "video",
            "data": "dmlkZW8=",
            "mimeType": "video/mp4"
        }))
        .is_err()
    );

    // Tags are zero-sized, so a mismatched tag cannot be built; deserializing
    // each struct must still require its own literal.
    let cases: [(&str, Value, Accepts); 5] = [
        (
            "text",
            serde_json::json!({"text": "t"}),
            accepts::<TextContent>,
        ),
        (
            "image",
            serde_json::json!({"data": "aGk=", "mimeType": "image/png"}),
            accepts::<ImageContent>,
        ),
        (
            "audio",
            serde_json::json!({"data": "aGk=", "mimeType": "audio/wav"}),
            accepts::<AudioContent>,
        ),
        (
            "resource_link",
            serde_json::json!({"name": "n", "uri": "file:///n"}),
            accepts::<ResourceLink>,
        ),
        (
            "resource",
            serde_json::json!({"resource": {"uri": "file:///r", "text": "r"}}),
            accepts::<EmbeddedResource>,
        ),
    ];
    for (tag, fields, accepts) in cases {
        let with_tag = |tag: Option<Value>| {
            let mut fields = fields.as_object().unwrap().clone();
            if let Some(tag) = tag {
                fields.insert("type".into(), tag);
            }
            Value::Object(fields)
        };
        assert!(
            accepts(&with_tag(Some(tag.into()))),
            "{tag} rejected its tag"
        );
        assert!(!accepts(&with_tag(None)), "{tag} accepted a missing tag");
        for wrong in [
            "text",
            "image",
            "audio",
            "resource_link",
            "resource",
            "video",
            "Text",
            "",
            "resource_link ",
        ] {
            if wrong != tag {
                assert!(
                    !accepts(&with_tag(Some(wrong.into()))),
                    "{tag} accepted {wrong:?}"
                );
            }
        }
        for wrong in [Value::Null, Value::from(1), serde_json::json!([tag])] {
            assert!(
                !accepts(&with_tag(Some(wrong.clone()))),
                "{tag} accepted {wrong}"
            );
        }
        // A block keyed by this tag must also have this variant's shape.
        let block = with_tag(Some(tag.into()));
        assert_eq!(
            serde_json::from_value::<ContentBlock>(block)
                .unwrap()
                .into_value()["type"],
            tag
        );
    }
    assert!(
        serde_json::from_value::<ContentBlock>(serde_json::json!({
            "type": "text",
            "data": "aGk=",
            "mimeType": "image/png"
        }))
        .is_err()
    );
}

#[test]
fn icon_src_must_be_an_http_https_or_data_uri() {
    use crate::test_support::decodes;
    type Rejection = Option<fn(String) -> IconError>;
    let cases: [(&str, Rejection); 11] = [
        ("https://example.com/i.png", None),
        ("http://example.com/i.png", None),
        ("HTTPS://example.com/i.png", None),
        ("data:image/png;base64,aWNvbg==", None),
        ("javascript:alert(1)", Some(IconError::UnsafeScheme)),
        ("file:///etc/icon.png", Some(IconError::UnsafeScheme)),
        ("ftp://example.com/i.png", Some(IconError::UnsafeScheme)),
        ("ws://example.com/i.png", Some(IconError::UnsafeScheme)),
        ("i", Some(IconError::InvalidSrc)),
        ("", Some(IconError::InvalidSrc)),
        ("https://exa mple.com/i.png", Some(IconError::InvalidSrc)),
    ];
    for (src, error) in cases {
        match error {
            None => assert_eq!(Icon::new(src).unwrap().src.as_str(), src),
            Some(error) => assert_eq!(Icon::new(src), Err(error(src.to_owned()))),
        }
        let wire = serde_json::json!({"src": src});
        assert_eq!(decodes::<Icon>(&wire), error.is_none(), "{src:?}");
    }
}

#[test]
fn icon_sizes_are_wxh_or_any() {
    use crate::test_support::decodes;
    let pixels = |width, height| Some(IconSize::Pixels { width, height });
    let cases = [
        ("48x48", pixels(48, 48)),
        ("96x64", pixels(96, 64)),
        ("any", Some(IconSize::Any)),
        ("48", None),
        ("48X48", None),
        ("x48", None),
        ("48x", None),
        ("+48x48", None),
        ("48x48 ", None),
        ("4294967296x1", None),
        ("Any", None),
        ("", None),
    ];
    for (size, expected) in cases {
        assert_eq!(size.parse::<IconSize>().ok(), expected, "{size:?}");
        let wire = serde_json::json!({"src": "https://example.com/i.png", "sizes": [size]});
        assert_eq!(decodes::<Icon>(&wire), expected.is_some(), "{size:?}");
        if let Some(parsed) = expected {
            assert_eq!(serde_json::to_value(parsed).unwrap(), size);
        }
    }
}

#[test]
fn icon_into_value_matches_serde_for_every_theme() {
    for theme in [IconTheme::Light, IconTheme::Dark] {
        let icon = Icon::new("https://example.com/i.png")
            .unwrap()
            .with_theme(theme);
        let expected = serde_json::to_value(&icon).unwrap();
        assert_eq!(icon.into_value(), expected, "theme: {theme:?}");
    }
}
