use serde_json::json;

use super::*;
use crate::content::annotations::Annotations;
use crate::content::resource::ResourceContent;
use crate::content::types::{Icon, ResourceLink};
use crate::protocol::types::Role;
use crate::test_support::priority;

#[test]
fn call_tool_result_roundtrips_complete_content_blocks() {
    let meta: serde_json::Map<String, Value> =
        serde_json::from_value(json!({"traceId": "trace-7"})).unwrap();
    let annotations = Annotations::new()
        .with_audience(vec![Role::User, Role::Assistant])
        .with_priority(priority(1.0));
    let resource_link = ResourceLink::named("guide", "file:///guide.md")
        .with_title("Guide")
        .with_description("Usage guide")
        .with_mime_type("text/markdown")
        .with_icons(vec![Icon::new("https://example.com/guide.png").unwrap()])
        .with_size(512)
        .with_annotations(annotations.clone())
        .with_meta(meta.clone());
    let result = CallToolResult {
        content: vec![
            ContentItem::text("hello")
                .with_annotations(annotations.clone())
                .with_meta(meta.clone()),
            ContentItem::image("aW1hZ2U=", "image/png").with_annotations(annotations.clone()),
            ContentItem::audio("YXVkaW8=", "audio/wav").with_annotations(annotations.clone()),
            resource_link.into(),
            ContentItem::embedded_resource(
                ResourceContent::blob("file:///image.png", "aW1hZ2U=", "image/png").with_meta(meta),
            )
            .with_annotations(annotations),
        ],
        is_error: None,
        structured_content: Some(serde_json::from_value(json!({"ok": true})).unwrap()),
        meta: Some(serde_json::from_value(json!({"result": "metadata"})).unwrap()),
    };

    let value = serde_json::to_value(&result).unwrap();
    assert_eq!(value["content"][0]["annotations"]["priority"], 1.0);
    assert_eq!(value["content"][0]["_meta"]["traceId"], "trace-7");
    assert_eq!(value["content"][1]["type"], "image");
    assert_eq!(value["content"][2]["type"], "audio");
    assert_eq!(value["content"][3]["description"], "Usage guide");
    assert_eq!(
        value["content"][4]["resource"]["_meta"]["traceId"],
        "trace-7"
    );

    assert_eq!(
        serde_json::from_value::<CallToolResult>(value).unwrap(),
        result
    );
}

#[test]
fn call_tool_result_rejects_non_object_structured_content() {
    assert!(
        serde_json::from_value::<CallToolResult>(json!({
            "content": [],
            "structuredContent": ["not", "an", "object"]
        }))
        .is_err()
    );
}

#[test]
fn call_tool_params_roundtrip_task_and_object_metadata() {
    let wire = json!({
        "name": "report",
        "arguments": {"format": "pdf"},
        "task": {"ttl": 30000},
        "_meta": {"progressToken": "report-1"}
    });
    let params: CallToolRequestParams = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(params.task.as_ref().unwrap().ttl, Some(30000));
    assert_eq!(serde_json::to_value(params).unwrap(), wire);

    let cases = [
        wire,
        json!({"name": "report", "arguments": null, "task": null, "_meta": null, "extra": 1}),
        json!(["report", {"format": "pdf"}]),
        json!({"name": "report", "_meta": ["not", "an", "object"]}),
        json!({"name": "report", "arguments": "pdf"}),
        json!({"name": 7}),
        json!({"arguments": {}}),
        json!({"arguments": [], "task": {"ttl": "soon"}}),
        json!({"name": "report", "task": {"ttl": "soon"}}),
        json!("report"),
    ];
    for case in cases {
        let decoded = |result: Result<CallToolRequestParams, serde_json::Error>| {
            result
                .map(|params| serde_json::to_value(params).unwrap())
                .map_err(|e| e.to_string())
        };
        assert_eq!(
            decoded(CallToolRequestParams::from_value(case.clone())),
            decoded(serde_json::from_value(case.clone())),
            "{case}"
        );
    }

    let format = "pdf".repeat(4096);
    let buffer = format.as_ptr();
    let mut wire = json!({"name": "report", "arguments": {}});
    wire["arguments"]["format"] = Value::String(format);
    let params = CallToolRequestParams::from_value(wire).unwrap();
    assert_eq!(
        params.arguments.unwrap()["format"]
            .as_str()
            .unwrap()
            .as_ptr(),
        buffer
    );
}

#[test]
fn cancelled_notification_allows_missing_request_id() {
    let params: CancelledNotificationParams =
        serde_json::from_value(json!({"reason": "task cancellation uses tasks/cancel"})).unwrap();
    assert!(params.request_id.is_none());
    assert_eq!(
        serde_json::to_value(params).unwrap(),
        json!({"reason": "task cancellation uses tasks/cancel"})
    );
}
