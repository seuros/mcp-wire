use super::*;
use crate::test_support::priority;

#[test]
fn test_text_builder() {
    let content = text("Hello, world!")
        .with_priority(priority(0.9))
        .with_audience(Role::User)
        .build();

    assert_eq!(content.text, "Hello, world!");
    assert!(content.annotations.is_some());

    let ann = content.annotations.unwrap();
    assert_eq!(ann.priority, Some(priority(0.9)));
    assert_eq!(ann.audience, Some(vec![Role::User]));
}

#[test]
fn test_text_builder_multiple_audiences() {
    let content = text("Important message")
        .with_audience(Role::User)
        .with_audience(Role::Assistant)
        .build();

    let ann = content.annotations.unwrap();
    assert_eq!(ann.audience, Some(vec![Role::User, Role::Assistant]));
}

#[test]
fn test_image_builder() {
    let content = image("base64data", "image/png")
        .with_priority(priority(0.8))
        .with_last_modified("2025-01-01T00:00:00Z")
        .build();

    assert_eq!(content.data, "base64data");
    assert_eq!(content.mime_type, "image/png");

    let ann = content.annotations.unwrap();
    assert_eq!(ann.priority, Some(priority(0.8)));
    assert_eq!(ann.last_modified, Some("2025-01-01T00:00:00Z".to_string()));
}

#[test]
fn test_audio_builder() {
    let content = audio("audiodata", "audio/wav")
        .with_priority(priority(0.7))
        .build();

    assert_eq!(content.data, "audiodata");
    assert_eq!(content.mime_type, "audio/wav");

    let ann = content.annotations.unwrap();
    assert_eq!(ann.priority, Some(priority(0.7)));
}

#[test]
fn test_resource_builder() {
    let link = resource("file://test.txt")
        .with_name("test-file")
        .with_title("Test File")
        .with_priority(priority(0.9))
        .build();

    assert_eq!(link.uri, "file://test.txt");
    assert_eq!(link.name, "test-file");
    assert_eq!(link.r#type.as_str(), "resource_link");
    assert_eq!(link.title, Some("Test File".to_string()));

    let ann = link.annotations.unwrap();
    assert_eq!(ann.priority, Some(priority(0.9)));

    let link = resource("file://test.txt").build();
    assert_eq!(link.name, "file://test.txt");
}

#[test]
fn test_builder_without_annotations() {
    let content = text("Plain text").build();

    assert_eq!(content.text, "Plain text");
    assert!(content.annotations.is_none());
}

#[test]
fn test_builder_type_usage() {
    let builder = TextContentBuilder::new("test");
    let content = builder.with_priority(priority(0.5)).build();

    assert_eq!(content.text, "test");
    assert_eq!(content.annotations.unwrap().priority, Some(priority(0.5)));
}
