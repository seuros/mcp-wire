use super::*;

#[test]
fn test_error_builder() {
    let error = McpError::builder(ErrorType::Validation, "TEST_ERROR")
        .message("Test error message")
        .detail("field", "value")
        .build();

    assert_eq!(error.error_type, ErrorType::Validation);
    assert_eq!(error.code, "TEST_ERROR");
    assert_eq!(error.message, "Test error message");
    assert_eq!(
        error.details.get("field").unwrap(),
        &Value::String("value".to_string())
    );
}

#[test]
fn test_to_jsonrpc() {
    let error = McpError::validation("INVALID_PARAM", "Parameter X is invalid");
    let json_err = error.to_jsonrpc();

    assert_eq!(json_err.code, -32602);
    assert_eq!(json_err.message, "Parameter X is invalid");
}

#[test]
fn consuming_conversion_preserves_codes_and_wire_details() {
    for error_type in [
        ErrorType::Validation,
        ErrorType::NotFound,
        ErrorType::MethodNotFound,
        ErrorType::Unauthorized,
        ErrorType::Internal,
        ErrorType::Timeout,
        ErrorType::Cancelled,
        ErrorType::RateLimit,
        ErrorType::Conflict,
        ErrorType::CircuitOpen,
    ] {
        let error = McpError::builder(error_type, "custom_code")
            .message("message")
            .detail("type", "must be overwritten")
            .detail("code", "must be overwritten")
            .detail("nested", serde_json::json!({"array": [null, 42, "text"]}))
            .build();
        let expected = serde_json::to_value(error.to_jsonrpc()).unwrap();
        assert_eq!(
            serde_json::to_value(error.into_jsonrpc()).unwrap(),
            expected
        );
        assert_eq!(expected["data"]["type"], error_type.to_string());
        assert_eq!(expected["data"]["code"], "custom_code");
    }
}

#[test]
fn consuming_conversion_moves_message_code_and_detail_buffers() {
    let message = "message".repeat(4096);
    let code = "code".repeat(4096);
    let detail = "detail".repeat(4096);
    let key = "detail_key".repeat(1024);
    let pointers = [
        message.as_ptr(),
        code.as_ptr(),
        detail.as_ptr(),
        key.as_ptr(),
    ];
    let error = McpError::builder(ErrorType::Internal, code)
        .message(message)
        .detail(key, detail)
        .build();
    let rpc = error.into_jsonrpc();
    assert_eq!(rpc.message.as_ptr(), pointers[0]);
    let data = rpc.data.as_ref().unwrap().as_object().unwrap();
    assert_eq!(data["code"].as_str().unwrap().as_ptr(), pointers[1]);
    let (key, value) = data
        .iter()
        .find(|(key, _)| key.starts_with("detail_key"))
        .unwrap();
    assert_eq!(value.as_str().unwrap().as_ptr(), pointers[2]);
    assert_eq!(key.as_ptr(), pointers[3]);
}

#[test]
fn borrowed_conversion_keeps_independent_details() {
    let error = McpError::builder(ErrorType::Internal, "code")
        .message("message")
        .detail("nested", serde_json::json!({"text": "original"}))
        .build();
    let mut rpc = error.to_jsonrpc();
    rpc.data.as_mut().unwrap()["nested"]["text"] = "changed".into();
    assert_eq!(error.details["nested"]["text"], "original");
    assert_eq!(error.message, "message");
    assert_eq!(error.code, "code");
}

#[test]
fn test_convenience_constructors() {
    let err = McpError::not_found("TEST", "not found");
    assert_eq!(err.error_type, ErrorType::NotFound);

    let err = McpError::internal("TEST", "internal");
    assert_eq!(err.error_type, ErrorType::Internal);
}
