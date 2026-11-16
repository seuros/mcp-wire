use super::*;
use serde_json::Value;
use std::assert_matches;

#[test]
fn test_version_gte_equal() {
    assert!(version_gte("1.0.0", "1.0.0"));
    assert!(version_gte("2.0.56", "2.0.56"));
}

#[test]
fn test_version_gte_major_greater() {
    assert!(version_gte("2.0.0", "1.0.0"));
    assert!(version_gte("3.0.0", "2.9.99"));
}

#[test]
fn test_version_gte_minor_greater() {
    assert!(version_gte("1.2.0", "1.1.0"));
    assert!(version_gte("1.10.0", "1.9.0"));
}

#[test]
fn test_version_gte_patch_greater() {
    assert!(version_gte("1.0.2", "1.0.1"));
    assert!(version_gte("2.0.56", "2.0.55"));
}

#[test]
fn test_version_gte_less_than() {
    assert!(!version_gte("1.0.0", "2.0.0"));
    assert!(!version_gte("2.0.55", "2.0.56"));
    assert!(!version_gte("1.9.0", "2.0.0"));
}

#[test]
fn test_jsonrpc_request_creation() {
    let req = JsonRpcRequest::new(
        Value::from(1),
        "test_method",
        Some(serde_json::json!({"key": "value"})),
    );
    assert_eq!(req.jsonrpc.as_str(), "2.0");
    assert_eq!(req.method, "test_method");
    assert!(req.id.is_some());
}

#[test]
fn test_jsonrpc_notification() {
    let notif = JsonRpcRequest::notification("test_notif", None);
    assert!(notif.is_notification());
    assert!(notif.id.is_none());
}

#[test]
fn test_jsonrpc_response_success() {
    let resp = JsonRpcResponse::success(Value::from(1), serde_json::json!({"status": "ok"}));
    assert!(resp.result.is_some());
    assert!(resp.error.is_none());
}

#[test]
fn test_jsonrpc_response_error() {
    let error = JsonRpcError::method_not_found("test method not found");
    let resp = JsonRpcResponse::error(Some(Value::from(1)), error);
    assert!(resp.result.is_none());
    assert!(resp.error.is_some());
    assert_eq!(resp.error.unwrap().code, ErrorCode::METHOD_NOT_FOUND);
}

#[test]
fn test_request_id_hash_eq() {
    use std::collections::HashMap;
    let mut map = HashMap::new();
    map.insert(RequestId::number(1), "first");
    map.insert(RequestId::string("abc"), "second");
    assert_eq!(map.get(&RequestId::number(1)), Some(&"first"));
    assert_eq!(map.get(&RequestId::string("abc")), Some(&"second"));
    assert_eq!(map.get(&RequestId::number(2)), None);
}

#[test]
fn test_request_id_display() {
    assert_eq!(RequestId::number(42).to_string(), "42");
    assert_eq!(RequestId::string("req-1").to_string(), "req-1");
}

#[test]
fn test_request_id_serde_roundtrip() {
    let id_num = RequestId::number(42);
    let json = serde_json::to_value(&id_num).unwrap();
    assert_eq!(json, serde_json::json!(42));
    let back: RequestId = serde_json::from_value(json).unwrap();
    assert_eq!(back, id_num);

    let id_str = RequestId::string("req-1");
    let json = serde_json::to_value(&id_str).unwrap();
    assert_eq!(json, serde_json::json!("req-1"));
    let back: RequestId = serde_json::from_value(json).unwrap();
    assert_eq!(back, id_str);

    let large = RequestId::from_value(&serde_json::json!(u64::MAX)).unwrap();
    assert_eq!(
        serde_json::to_value(large).unwrap(),
        serde_json::json!(u64::MAX)
    );
    assert!(RequestId::from_value(&serde_json::json!(42.5)).is_none());
}

#[test]
fn test_progress_token_serde_roundtrip() {
    let string_token = ProgressToken::from("request-1");
    let number_token = ProgressToken::from(42_i64);

    assert_eq!(
        serde_json::to_value(&string_token).unwrap(),
        serde_json::json!("request-1")
    );
    assert_eq!(
        serde_json::to_value(&number_token).unwrap(),
        serde_json::json!(42)
    );
    assert_eq!(
        ProgressToken::from_value(&serde_json::json!(42)),
        Some(number_token)
    );
    let fractional = ProgressToken::from_value(&serde_json::json!(0.5)).unwrap();
    assert_eq!(
        serde_json::to_value(fractional).unwrap(),
        serde_json::json!(0.5)
    );
}

#[test]
fn test_request_id_from_value() {
    assert_eq!(
        RequestId::from_value(&serde_json::json!(42)),
        Some(RequestId::number(42))
    );
    assert_eq!(
        RequestId::from_value(&serde_json::json!("abc")),
        Some(RequestId::string("abc"))
    );
    assert_eq!(RequestId::from_value(&serde_json::json!(null)), None);
    assert_eq!(RequestId::from_value(&serde_json::json!([1, 2])), None);
}

#[test]
fn test_jsonrpc_message_deserialize_request() {
    let json = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {"name": "echo"}
    });
    let msg: JsonRpcMessage = serde_json::from_value(json).unwrap();
    assert_matches!(msg, JsonRpcMessage::Request(_));
    assert_eq!(msg.method(), Some("tools/call"));
}

#[test]
fn test_jsonrpc_message_deserialize_notification() {
    let json = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });
    let msg: JsonRpcMessage = serde_json::from_value(json).unwrap();
    assert_matches!(msg, JsonRpcMessage::Notification(_));
    assert!(msg.id().is_none());
}

#[test]
fn test_jsonrpc_message_deserialize_response() {
    let json = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {"status": "ok"}
    });
    let msg: JsonRpcMessage = serde_json::from_value(json).unwrap();
    assert_matches!(msg, JsonRpcMessage::Response(_));
}

#[test]
fn test_jsonrpc_rejects_fractional_numeric_ids() {
    let request_json = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1.5,
        "method": "ping"
    });
    assert!(serde_json::from_value::<JsonRpcRequest>(request_json).is_err());

    let response_json = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1.5,
        "result": {}
    });
    assert!(serde_json::from_value::<JsonRpcResponse>(response_json).is_err());
}

#[test]
fn test_jsonrpc_rejects_invalid_version_and_request_id() {
    let wrong_version = serde_json::json!({
        "jsonrpc": "1.0",
        "id": 1,
        "method": "ping"
    });
    let boolean_id = serde_json::json!({
        "jsonrpc": "2.0",
        "id": true,
        "method": "ping"
    });
    let null_id = serde_json::json!({
        "jsonrpc": "2.0",
        "id": null,
        "method": "ping"
    });
    let scalar_params = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "ping",
        "params": true
    });
    let array_params = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "ping",
        "params": []
    });

    assert!(serde_json::from_value::<JsonRpcRequest>(wrong_version).is_err());
    assert!(serde_json::from_value::<JsonRpcRequest>(boolean_id).is_err());
    assert!(serde_json::from_value::<JsonRpcRequest>(null_id).is_err());
    assert!(serde_json::from_value::<JsonRpcRequest>(scalar_params).is_err());
    assert!(serde_json::from_value::<JsonRpcRequest>(array_params).is_err());
}

#[test]
fn test_jsonrpc_response_requires_exactly_one_payload() {
    let both = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {},
        "error": {"code": -32603, "message": "failure"}
    });
    let neither = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1
    });
    let null_result = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": null
    });
    let scalar_result = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": true
    });
    let array_result = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": []
    });
    let missing_success_id = serde_json::json!({
        "jsonrpc": "2.0",
        "result": {}
    });
    let unidentified_error = serde_json::json!({
        "jsonrpc": "2.0",
        "id": null,
        "error": {"code": -32700, "message": "parse error"}
    });
    let missing_error_id = serde_json::json!({
        "jsonrpc": "2.0",
        "error": {"code": -32700, "message": "parse error"}
    });

    assert!(serde_json::from_value::<JsonRpcResponse>(both).is_err());
    assert!(serde_json::from_value::<JsonRpcResponse>(neither).is_err());
    assert!(serde_json::from_value::<JsonRpcResponse>(missing_success_id).is_err());
    assert!(serde_json::from_value::<JsonRpcResponse>(null_result).is_err());
    assert!(serde_json::from_value::<JsonRpcResponse>(scalar_result).is_err());
    assert!(serde_json::from_value::<JsonRpcResponse>(array_result).is_err());
    assert!(serde_json::from_value::<JsonRpcResponse>(unidentified_error).is_err());
    let error = serde_json::from_value::<JsonRpcResponse>(missing_error_id).unwrap();
    assert!(error.id.is_none());
    assert!(serde_json::to_value(error).unwrap().get("id").is_none());
}

#[test]
fn test_jsonrpc_error_object_decodes_like_serde() {
    let cases = [
        serde_json::json!({"code": -32603, "message": "failure", "data": {"retry": [1, 2]}}),
        serde_json::json!({"code": -32603, "message": "failure", "data": null, "extra": 1}),
        serde_json::json!([-32603, "failure"]),
        serde_json::json!({"message": "failure"}),
        serde_json::json!({"code": -32603}),
        serde_json::json!({"code": "bad", "message": 7}),
        serde_json::json!({"code": 4_294_967_296_i64, "message": "failure"}),
        serde_json::json!("failure"),
    ];
    for case in cases {
        let response = serde_json::json!({"jsonrpc": "2.0", "id": 1, "error": case});
        assert_eq!(
            JsonRpcResponse::from_value(response)
                .map(|response| response.error.unwrap())
                .map_err(|error| error.to_string()),
            serde_json::from_value::<JsonRpcError>(case.clone()).map_err(|error| error.to_string()),
            "{case}"
        );
    }

    let data = "detail".repeat(4096);
    let buffer = data.as_ptr();
    let mut response = serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "error": {"code": -32603, "message": "failure", "data": {}}
    });
    response["error"]["data"]["detail"] = Value::String(data);
    let error = JsonRpcResponse::from_value(response)
        .unwrap()
        .error
        .unwrap();
    assert_eq!(
        error.data.unwrap()["detail"].as_str().unwrap().as_ptr(),
        buffer
    );
}

#[test]
fn test_jsonrpc_serialization_rejects_invalid_public_literal() {
    let request = JsonRpcRequest {
        jsonrpc: JsonRpcVersion,
        id: Some(serde_json::json!(false)),
        method: "ping".to_string(),
        params: None,
    };
    let array_params =
        JsonRpcRequest::new(serde_json::json!(1), "ping", Some(serde_json::json!([])));
    let response = JsonRpcResponse {
        jsonrpc: JsonRpcVersion,
        id: Some(serde_json::json!(1)),
        result: Some(serde_json::json!({})),
        error: Some(JsonRpcError::internal_error("failure")),
    };
    let unidentified_success = JsonRpcResponse::success(Value::Null, serde_json::json!({}));
    let scalar_success = JsonRpcResponse::success(serde_json::json!(1), Value::Bool(true));
    let unidentified_error =
        JsonRpcResponse::error(None, JsonRpcError::invalid_request("invalid request"));
    let null_error = JsonRpcResponse::error(
        Some(Value::Null),
        JsonRpcError::invalid_request("invalid request"),
    );

    assert!(serde_json::to_value(request).is_err());
    assert!(serde_json::to_value(array_params).is_err());
    assert!(serde_json::to_value(response).is_err());
    assert!(serde_json::to_value(unidentified_success).is_err());
    assert!(serde_json::to_value(scalar_success).is_err());
    assert!(
        serde_json::to_value(unidentified_error)
            .unwrap()
            .get("id")
            .is_none()
    );
    assert!(serde_json::to_value(null_error).is_err());
}

#[test]
fn test_content_item_text() {
    let item = ContentItem::text("hello");
    let json = serde_json::to_value(&item).unwrap();
    assert_eq!(json["type"], "text");
    assert_eq!(json["text"], "hello");
}

#[test]
fn test_content_item_roundtrip() {
    let item = ContentItem::image("base64data", "image/png");
    let json = serde_json::to_value(&item).unwrap();
    let back: ContentItem = serde_json::from_value(json).unwrap();
    assert_eq!(back, item);
}

#[test]
fn test_content_item_resource_link() {
    let item = ContentItem::resource("file://test.txt");
    let json = serde_json::to_value(&item).unwrap();

    assert_eq!(json["type"], "resource_link");
    assert_eq!(json["name"], "file://test.txt");
    assert_eq!(json["uri"], "file://test.txt");
}

#[test]
fn test_content_item_embedded_resource() {
    let item = ContentItem::embedded_resource(crate::content::resource::ResourceContent::text(
        "file://test.txt",
        "Hello",
    ));
    let json = serde_json::to_value(&item).unwrap();

    assert_eq!(json["type"], "resource");
    assert_eq!(json["resource"]["uri"], "file://test.txt");
    assert_eq!(json["resource"]["text"], "Hello");
}

#[test]
fn test_call_tool_result_text() {
    let result = CallToolResult::text("done");
    assert_eq!(result.content.len(), 1);
    assert!(result.is_error.is_none());
}

#[test]
fn test_call_tool_result_error() {
    let result = CallToolResult::error("failed");
    assert_eq!(result.is_error, Some(true));
}

#[test]
fn test_completion_values_serialize() {
    let result = CompleteResult {
        meta: None,
        completion: CompletionInfo::with_values(vec![
            CompletionValue::new("Alice"),
            CompletionValue::new("Amy"),
        ]),
    };
    let json = serde_json::to_value(result).unwrap();

    assert_eq!(
        json["completion"]["values"],
        serde_json::json!(["Alice", "Amy"])
    );

    let decoded: CompleteResult = serde_json::from_value(json).unwrap();
    assert_eq!(decoded.completion.values[0].as_str(), "Alice");
    assert!(
        serde_json::from_value::<CompleteResult>(serde_json::json!({
            "completion": {"values": [{"value": "legacy-object"}]}
        }))
        .is_err()
    );
}

/// `count` distinct completion values.
fn completion_values(count: usize) -> Vec<CompletionValue> {
    (0..count)
        .map(|index| CompletionValue::new(index.to_string()))
        .collect()
}

#[test]
fn test_completion_values_are_capped_on_the_wire() {
    let result = CompleteResult {
        meta: None,
        completion: CompletionInfo {
            values: completion_values(101),
            total: None,
            has_more: None,
        },
    };
    let json = serde_json::to_value(result).unwrap();

    assert_eq!(json["completion"]["values"].as_array().unwrap().len(), 100);
    assert_eq!(json["completion"]["hasMore"], true);
}

#[test]
fn completion_total_is_never_below_the_values_held() {
    let cases = [
        (
            CompletionInfo::with_pagination(completion_values(5), 3, false),
            Some(5),
        ),
        (
            CompletionInfo::with_pagination(completion_values(150), 3, true),
            Some(150),
        ),
        (
            CompletionInfo::with_pagination(completion_values(5), 10, true),
            Some(10),
        ),
        (
            CompletionInfo {
                values: completion_values(4),
                total: Some(2),
                has_more: None,
            },
            Some(4),
        ),
        (
            CompletionInfo {
                values: completion_values(4),
                total: None,
                has_more: None,
            },
            None,
        ),
    ];
    for (info, total) in cases {
        let total = total.map(Value::from);
        let wire = serde_json::to_value(&info).unwrap();
        assert_eq!(wire.get("total"), total.as_ref(), "{info:?}");
        assert_eq!(info.into_value(), wire);
    }
}

#[test]
fn test_completion_request_preserves_context_and_metadata() {
    let wire = serde_json::json!({
        "ref": {
            "type": "ref/prompt",
            "name": "deploy",
            "title": "Deploy service"
        },
        "argument": {"name": "region", "value": "eu-"},
        "context": {"arguments": {"environment": "production"}},
        "_meta": {"traceId": "trace-1"}
    });

    let request: CompleteRequest = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(request).unwrap(), wire);
}

#[test]
fn test_create_message_result_is_flat() {
    let result = CreateMessageResult {
        role: Role::Assistant,
        content: SamplingContent::text("hello").into(),
        model: "test-model".to_string(),
        stop_reason: Some(CreateMessageResult::STOP_REASON_END_TURN.to_string()),
        meta: None,
    };
    let json = serde_json::to_value(result).unwrap();

    assert_eq!(json["role"], "assistant");
    assert_eq!(json["content"]["text"], "hello");
    assert_eq!(json["model"], "test-model");
    assert!(json.get("message").is_none());
}

#[test]
fn test_implementation_builder() {
    let impl_info = Implementation::new("test", "1.0")
        .with_title("Test Server")
        .with_website_url("https://example.com".parse().unwrap());
    assert_eq!(impl_info.name, "test");
    assert_eq!(impl_info.title, Some("Test Server".to_string()));
    assert_eq!(
        impl_info.website_url.as_ref().map(WebsiteUrl::as_str),
        Some("https://example.com")
    );
    let minimal = Implementation::new("test", "1.0");
    let json = serde_json::to_value(&minimal).unwrap();
    assert!(!json.as_object().unwrap().contains_key("title"));
}

#[test]
fn test_error_code_constants() {
    assert_eq!(ErrorCode::PARSE_ERROR, -32700);
    assert_eq!(ErrorCode::INVALID_REQUEST, -32600);
    assert_eq!(ErrorCode::METHOD_NOT_FOUND, -32601);
    assert_eq!(ErrorCode::INVALID_PARAMS, -32602);
    assert_eq!(ErrorCode::INTERNAL_ERROR, -32603);
    assert_eq!(ErrorCode::SERVER_ERROR, -32000);
    assert_eq!(ErrorCode::UNAUTHORIZED, -32001);
    assert_eq!(ErrorCode::RESOURCE_NOT_FOUND, -32002);
}
