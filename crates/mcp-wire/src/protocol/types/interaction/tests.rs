use serde_json::json;

use super::*;
use crate::test_support::decodes;
use std::assert_matches;

#[test]
fn sampling_request_roundtrips_typed_extensions() {
    let wire = json!({
        "messages": [
            {
                "role": "user",
                "content": {
                    "type": "text",
                    "text": "weather",
                    "annotations": {"audience": ["user"], "priority": 0.5}
                },
                "_meta": {"messageId": "message-1"}
            },
            {
                "role": "assistant",
                "content": [
                    {"type": "image", "data": "AAAA", "mimeType": "image/png"},
                    {"type": "audio", "data": "BBBB", "mimeType": "audio/wav", "_meta": {"k": 1}},
                    {"type": "tool_use", "id": "call_1", "name": "weather", "input": {"city": "Paris"}}
                ]
            },
            {
                "role": "user",
                "content": {
                    "type": "tool_result",
                    "toolUseId": "call_1",
                    "content": [
                        {"type": "text", "text": "sunny"},
                        {"type": "resource", "resource": {"uri": "file:///w", "text": "x"}}
                    ],
                    "structuredContent": {"celsius": 20},
                    "isError": false,
                    "_meta": {"cacheKey": "result-1"}
                }
            }
        ],
        "systemPrompt": "be brief",
        "temperature": 0.5,
        "stopSequences": ["END"],
        "modelPreferences": {"hints": [{"name": "claude"}], "costPriority": 0.2},
        "includeContext": "allServers",
        "maxTokens": 256,
        "metadata": {"provider": "test"},
        "tools": [{
            "name": "weather",
            "title": "Weather",
            "description": "Fetch the weather",
            "inputSchema": {"type": "object"},
            "icons": [{"src": "https://example.com/weather.png"}],
            "outputSchema": {"type": "object"},
            "execution": {"taskSupport": "optional"},
            "annotations": {"readOnlyHint": true},
            "_meta": {"revision": 2}
        }],
        "toolChoice": {},
        "task": {"ttl": 5000},
        "_meta": {"traceId": "trace-1"}
    });

    let params: CreateMessageParams = serde_json::from_value(wire.clone()).unwrap();
    assert_matches!(params.messages[0].role, Role::User);
    assert_eq!(params.include_context, Some(ContextInclusion::AllServers));
    assert_eq!(
        params.tool_choice.as_ref().unwrap().mode,
        ToolChoiceMode::Auto
    );
    assert_eq!(params.task.as_ref().unwrap().ttl, Some(5000));
    assert_eq!(
        params.tools.as_ref().unwrap()[0].title.as_deref(),
        Some("Weather")
    );
    assert_eq!(params.clone().into_value().unwrap(), wire);
    assert_eq!(serde_json::to_value(params).unwrap(), wire);
}

#[test]
fn sampling_conversions_move_payloads() {
    let text = "prompt".repeat(4096);
    let buffer = text.as_ptr();
    let params = CreateMessageParams {
        messages: vec![SamplingMessage {
            role: Role::User,
            content: SamplingContent::text(text).into(),
            meta: None,
        }],
        max_tokens: 1,
        ..Default::default()
    };
    let wire = params.into_value().unwrap();
    assert_eq!(
        wire["messages"][0]["content"]["text"]
            .as_str()
            .unwrap()
            .as_ptr(),
        buffer
    );

    for content in [json!({"type": "text"}), json!([{"type": "text"}])] {
        let text = "reply".repeat(4096);
        let buffer = text.as_ptr();
        let mut wire = json!({"role": "assistant", "content": content, "model": "m"});
        let block = if wire["content"].is_array() {
            &mut wire["content"][0]
        } else {
            &mut wire["content"]
        };
        block["text"] = Value::String(text);
        let result: CreateMessageResult = serde_json::from_value(wire).unwrap();
        let block = match &result.content {
            SamplingMessageContent::Single(block) => block,
            SamplingMessageContent::Multiple(blocks) => &blocks[0],
        };
        assert_matches!(block, SamplingContent::Text { text, .. } if text.as_ptr() == buffer);
    }
}

#[test]
fn sampling_priorities_lie_between_zero_and_one() {
    // schema.ts: every priority is `@minimum 0 @maximum 1`.
    let params = |preferences: Value, annotations: Value| {
        json!({
            "messages": [{
                "role": "user",
                "content": {"type": "text", "text": "hi", "annotations": annotations}
            }],
            "modelPreferences": preferences,
            "maxTokens": 1
        })
    };
    for (priority, valid) in [
        (json!(0), true),
        (json!(0.5), true),
        (json!(1.0), true),
        (json!(-0.1), false),
        (json!(1.5), false),
    ] {
        for field in ["costPriority", "speedPriority", "intelligencePriority"] {
            let wire = params(json!({ field: priority }), json!({}));
            assert_eq!(decodes::<CreateMessageParams>(&wire), valid, "{wire}");
        }
        let wire = params(json!({}), json!({"priority": priority}));
        assert_eq!(decodes::<CreateMessageParams>(&wire), valid, "{wire}");
    }
}

#[test]
fn sampling_request_rejects_unknown_role_and_context() {
    let base = json!({
        "messages": [{
            "role": "system",
            "content": {"type": "text", "text": "hello"}
        }],
        "maxTokens": 1
    });
    assert!(serde_json::from_value::<CreateMessageParams>(base).is_err());

    let invalid_context = json!({
        "messages": [],
        "includeContext": "local",
        "maxTokens": 1
    });
    assert!(serde_json::from_value::<CreateMessageParams>(invalid_context).is_err());
}

#[test]
fn sampling_text_joins_text_blocks_and_skips_media() {
    let cases = [
        (SamplingContent::text("one").into(), "one"),
        (
            vec![
                SamplingContent::text("one"),
                SamplingContent::image("AAAA", "image/png"),
                SamplingContent::text("two"),
            ]
            .into(),
            "one\ntwo",
        ),
        (SamplingMessageContent::Multiple(Vec::new()), ""),
    ];
    for (content, text) in cases {
        assert_eq!(content.text(), text, "{content:?}");
        assert_eq!(content.into_text(), text);
    }

    let text = "reply".repeat(4096);
    let buffer = text.as_ptr();
    let text = SamplingMessageContent::from(SamplingContent::text(text)).into_text();
    assert_eq!(text.as_ptr(), buffer);
}

#[test]
fn sampling_validate_enforces_the_tool_use_rules() {
    let tool_use = |id: &str| json!({"type": "tool_use", "id": id, "name": "weather", "input": {}});
    let tool_result = |id: &str| json!({"type": "tool_result", "toolUseId": id, "content": []});
    let text = json!({"type": "text", "text": "weather?"});
    let user = |content: Value| json!({"role": "user", "content": content});
    let assistant = |content: Value| json!({"role": "assistant", "content": content});
    let unanswered = "the next message must answer each tool use with one tool result";
    let cases = [
        // Parallel tool uses answered in any order, then the reply.
        (
            vec![
                user(text.clone()),
                assistant(json!([tool_use("a"), tool_use("b")])),
                user(json!([tool_result("b"), tool_result("a")])),
                assistant(text.clone()),
            ],
            None,
        ),
        (vec![assistant(tool_use("a")), user(tool_result("a"))], None),
        (
            vec![user(tool_use("a"))],
            Some("message 0: tool_use content belongs in an assistant message".into()),
        ),
        (
            vec![assistant(json!([tool_use("a"), tool_result("a")]))],
            Some("message 0: tool_result content belongs in a user message".into()),
        ),
        (
            vec![
                assistant(tool_use("a")),
                user(json!([text.clone(), tool_result("a")])),
            ],
            Some("message 1: a message with tool results must hold only tool results".into()),
        ),
        // Tool uses left unanswered, or answered too late.
        (
            vec![
                assistant(json!([tool_use("a"), tool_use("b")])),
                user(tool_result("a")),
            ],
            Some(format!("message 0: {unanswered}")),
        ),
        (
            vec![assistant(tool_use("a")), user(tool_result("z"))],
            Some(format!("message 0: {unanswered}")),
        ),
        (
            vec![
                assistant(tool_use("a")),
                user(text.clone()),
                user(tool_result("a")),
            ],
            Some(format!("message 0: {unanswered}")),
        ),
        (
            vec![user(text), assistant(tool_use("a"))],
            Some(format!("message 1: {unanswered}")),
        ),
        (
            vec![user(tool_result("a"))],
            Some("message 0: tool results must answer tool uses of the previous message".into()),
        ),
    ];
    for (messages, expected) in cases {
        let params: CreateMessageParams =
            serde_json::from_value(json!({"messages": messages, "maxTokens": 1})).unwrap();
        assert_eq!(
            params.validate().err().map(|error| error.to_string()),
            expected,
            "{messages:?}"
        );
    }

    // Tool schemas need object roots, as registered tools do (schema.ts `Tool`).
    for (tool, expected) in [
        (
            json!({"name": "w", "inputSchema": {"type": "object"}}),
            None,
        ),
        (
            json!({"name": "w", "inputSchema": {"type": "string"}}),
            Some("tool 'w' input schema must be a JSON object with type \"object\""),
        ),
        (
            json!({"name": "w", "inputSchema": {"type": "object"}, "outputSchema": true}),
            Some("tool 'w' output schema must be a JSON object with type \"object\""),
        ),
    ] {
        let params: CreateMessageParams =
            serde_json::from_value(json!({"messages": [], "maxTokens": 1, "tools": [tool]}))
                .unwrap();
        assert_eq!(
            params
                .validate()
                .err()
                .map(|error| error.to_string())
                .as_deref(),
            expected
        );
    }
}
