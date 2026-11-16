use super::*;
use std::assert_matches;

#[test]
fn test_method_display() {
    assert_eq!(McpMethod::Initialize.to_string(), "initialize");
    assert_eq!(McpMethod::ResourcesList.to_string(), "resources/list");
    assert_eq!(McpMethod::ToolsCall.to_string(), "tools/call");
}

#[test]
fn test_method_from_string() {
    assert_eq!(McpMethod::from("initialize"), McpMethod::Initialize);
    assert_eq!(McpMethod::from("tools/call"), McpMethod::ToolsCall);
    assert_eq!(
        McpMethod::from("notifications/progress"),
        McpMethod::NotificationsProgress
    );
}

#[test]
fn test_custom_method() {
    let method = McpMethod::from("custom/method");
    assert_matches!(method, McpMethod::Custom(_));
    assert_eq!(method.to_string(), "custom/method");
}

#[test]
fn parse_names_only_standard_methods() {
    let cases = [
        ("initialize", Some(McpMethod::Initialize)),
        ("tools/call", Some(McpMethod::ToolsCall)),
        (
            "notifications/initialized",
            Some(McpMethod::NotificationsInitialized),
        ),
        ("custom/method", None),
        ("", None),
    ];
    for (name, expected) in cases {
        assert_eq!(McpMethod::parse(name), expected, "{name:?}");
    }
}
