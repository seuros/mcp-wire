use super::*;

#[test]
fn embedded_resource_uses_content_block_shape() {
    let message = PromptMessage::user_resource(ResourceContent::text("file:///a.txt", "A"));
    let value = serde_json::to_value(message).unwrap();

    assert_eq!(value["content"]["type"], "resource");
    assert_eq!(value["content"]["resource"]["uri"], "file:///a.txt");
    assert_eq!(value["content"]["resource"]["text"], "A");
    assert!(value["content"].get("uri").is_none());
}

#[test]
fn prompt_messages_roundtrip_audio_and_resource_links() {
    let messages = vec![
        PromptMessage::user_audio("YXVkaW8=", "audio/wav"),
        PromptMessage::assistant_resource_link(
            ResourceLink::named("guide", "file:///guide.md")
                .with_description("Guide")
                .with_mime_type("text/markdown"),
        ),
    ];

    let value = serde_json::to_value(&messages).unwrap();
    assert_eq!(value[0]["content"]["type"], "audio");
    assert_eq!(value[0]["content"]["mimeType"], "audio/wav");
    assert_eq!(value[1]["content"]["type"], "resource_link");
    assert_eq!(value[1]["content"]["name"], "guide");
    assert_eq!(value[1]["content"]["description"], "Guide");

    assert_eq!(
        serde_json::from_value::<Vec<PromptMessage>>(value).unwrap(),
        messages
    );
}

#[test]
fn into_value_matches_serde_for_every_role() {
    for role in [PromptRole::User, PromptRole::Assistant] {
        let message = PromptMessage::new(role, ContentBlock::text("hi"));
        let expected = serde_json::to_value(&message).unwrap();
        assert_eq!(message.into_value(), expected, "role: {role:?}");
    }
}
