use super::*;
use std::assert_matches;

#[test]
fn test_text_resource() {
    let content = ResourceContent::text("file:///test.txt", "Hello");

    assert_eq!(content.uri, "file:///test.txt");
    assert_eq!(content.mime_type.as_deref(), Some("text/plain"));
    assert_eq!(content.payload, ResourcePayload::Text("Hello".into()));
    assert_eq!(content.meta, None);
}

#[test]
fn test_text_resource_with_mime_type() {
    let content =
        ResourceContent::text("file:///data.json", "{}").with_mime_type("application/json");

    assert_eq!(content.mime_type.as_deref(), Some("application/json"));
}

#[test]
fn constant_mime_types_are_borrowed_not_allocated() {
    let cases = [
        (
            "text default",
            ResourceContent::text("file:///a", "a"),
            "text/plain",
        ),
        (
            "mcp app",
            ResourceContent::mcp_app("ui://a/b", "<!doctype html>"),
            crate::protocol::apps::MIME_TYPE,
        ),
        (
            "override",
            ResourceContent::text("file:///a", "{}").with_mime_type("application/json"),
            "application/json",
        ),
        (
            "blob",
            ResourceContent::blob("file:///a", "AA==", "image/png"),
            "image/png",
        ),
    ];
    for (label, content, expected) in cases {
        assert_matches!(
            content.mime_type,
            Some(Cow::Borrowed(mime)) if mime == expected,
            "{label}"
        );
    }

    // Runtime MIME types are still accepted, and move in without a copy.
    let runtime = format!("text/{}", "csv");
    let buffer = runtime.as_ptr();
    let content = ResourceContent::text("file:///a.csv", "a,b").with_mime_type(runtime);
    assert_matches!(&content.mime_type, Some(Cow::Owned(mime)) if mime.as_ptr() == buffer);

    // Deserialized MIME types are owned and round-trip unchanged.
    let decoded: ResourceContent = serde_json::from_value(content.to_value()).unwrap();
    assert_eq!(decoded, content);
}

#[test]
fn test_blob_resource() {
    let content = ResourceContent::blob("file:///img.png", "base64data", "image/png");

    assert_eq!(content.uri, "file:///img.png");
    assert_eq!(content.mime_type.as_deref(), Some("image/png"));
    assert_eq!(content.payload, ResourcePayload::Blob("base64data".into()));
}

#[test]
fn test_serialization_camel_case() {
    let content = ResourceContent::text("file:///test", "data").with_mime_type("text/plain");

    let value = content.to_value();
    let obj = value.as_object().unwrap();

    assert!(obj.contains_key("uri"));
    assert!(obj.contains_key("mimeType"));
    assert!(obj.contains_key("text"));
    assert!(!obj.contains_key("mime_type"));
}

#[test]
fn test_resource_content_preserves_meta() {
    let content = ResourceContent::text("file:///test", "data")
        .with_meta(serde_json::from_value(serde_json::json!({"trace": 7})).unwrap());
    let value = serde_json::to_value(&content).unwrap();
    assert_eq!(value["_meta"]["trace"], 7);
    assert_eq!(
        serde_json::from_value::<ResourceContent>(value).unwrap(),
        content
    );
}

#[test]
fn payload_is_exactly_one_of_text_or_blob() {
    use serde_json::json;
    let cases = [
        (
            json!({"uri": "file:///t", "text": "t"}),
            Some(ResourcePayload::Text("t".into())),
        ),
        (
            json!({"uri": "file:///b", "blob": "YmxvYg=="}),
            Some(ResourcePayload::Blob("YmxvYg==".into())),
        ),
        (json!({"uri": "file:///missing"}), None),
        (
            json!({"uri": "file:///both", "text": "t", "blob": "YmxvYg=="}),
            None,
        ),
    ];
    for (wire, payload) in cases {
        let decoded = serde_json::from_value::<ResourceContent>(wire.clone()).ok();
        assert_eq!(
            decoded.as_ref().map(|content| &content.payload),
            payload.as_ref(),
            "{wire}"
        );
        if let Some(content) = decoded {
            assert_eq!(serde_json::to_value(&content).unwrap(), wire);
            assert_eq!(content.into_value(), wire);
        }
    }
}
