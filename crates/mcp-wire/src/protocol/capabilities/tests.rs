use super::*;

#[test]
fn test_server_capabilities_default() {
    let caps = ServerCapabilities::default();
    assert!(caps.tools.is_none());
    assert!(caps.resources.is_none());
}

#[test]
fn test_server_capabilities_serialization() {
    let caps = ServerCapabilities {
        completions: Some(CompletionCapability {}),
        tools: Some(ToolsCapability {
            list_changed: Some(true),
        }),
        resources: Some(ResourcesCapability {
            subscribe: Some(true),
            list_changed: Some(true),
        }),
        ..Default::default()
    };

    let json = serde_json::to_value(&caps).unwrap();
    assert!(json["tools"].is_object());
    assert!(json["resources"].is_object());
    assert!(json["completions"].is_object());
    assert!(json["completion"].is_null());
    assert_eq!(json["tools"]["listChanged"], true);
    assert!(json["resources"]["listTemplates"].is_null());
}

#[test]
fn test_initialize_request_deserialization() {
    let json = serde_json::json!({
        "protocolVersion": "2025-11-25",
        "capabilities": {},
        "clientInfo": {
            "name": "test-client",
            "version": "1.0.0"
        }
    });

    let req: InitializeRequest = serde_json::from_value(json).unwrap();
    assert_eq!(req.protocol_version, "2025-11-25");
    assert_eq!(req.client_info.name, "test-client");
}

#[test]
fn test_elicitation_capability_modes() {
    let empty = ElicitationCapability::default();
    assert!(empty.supports_form());
    assert!(!empty.supports_url());

    let form_only = ElicitationCapability {
        form: Some(FormElicitationCapability::default()),
        url: None,
    };
    assert!(form_only.supports_form());
    assert!(!form_only.supports_url());

    let url_only = ElicitationCapability {
        form: None,
        url: Some(UrlElicitationCapability::default()),
    };
    assert!(!url_only.supports_form());
    assert!(url_only.supports_url());
}

#[test]
fn test_sampling_capability_subfeatures() {
    let json = serde_json::json!({
        "sampling": {
            "context": {},
            "tools": {}
        },
        "tasks": {
            "requests": {
                "sampling": {"createMessage": {}}
            }
        }
    });

    let caps: ClientCapabilities = serde_json::from_value(json).unwrap();
    assert!(caps.supports_sampling());
    assert!(caps.supports_sampling_context());
    assert!(caps.supports_sampling_tools());
    assert!(caps.supports_task_sampling());

    let basic = ClientCapabilities {
        sampling: Some(SamplingCapability::default()),
        ..Default::default()
    };
    assert!(basic.supports_sampling());
    assert!(!basic.supports_sampling_context());
    assert!(!basic.supports_sampling_tools());
    assert!(!basic.supports_task_sampling());
}
