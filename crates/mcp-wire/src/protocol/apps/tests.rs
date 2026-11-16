use super::*;
use crate::protocol::capabilities::ClientCapabilities;
use std::assert_matches;

fn client_caps(value: Value) -> ClientCapabilities {
    serde_json::from_value(value).expect("client capabilities")
}

fn csp_meta(connect: &str) -> Result<Map<String, Value>, AppsError> {
    UiResourceMeta {
        csp: Some(UiCsp {
            connect_domains: Some(vec![connect.into()]),
            ..Default::default()
        }),
        ..Default::default()
    }
    .into_meta()
}

#[test]
fn ui_uri_is_printable_ascii_after_scheme() {
    for uri in [
        "ui://x",
        "ui://weather/dashboard",
        "ui:///x",
        "ui://a?b=c#d",
    ] {
        assert!(is_ui_uri(uri), "{uri:?}");
    }
    for uri in [
        "",
        "ui://",
        "UI://x",
        "https://example.com/view",
        "ui://has space",
        "ui://tab\t",
        "ui://nul\0",
        "ui://del\u{7f}",
        "ui://zero\u{200b}width",
        "ui://nbsp\u{a0}",
    ] {
        assert!(!is_ui_uri(uri), "{uri:?}");
        assert_eq!(ToolUi::new(uri), Err(AppsError::InvalidUri(uri.into())));
    }
}

#[test]
fn tool_ui_serializes_under_meta_ui() {
    let meta = ToolUi::new("ui://weather/dashboard")
        .unwrap()
        .with_visibility([UiVisibility::Model, UiVisibility::App])
        .into_meta();

    assert_eq!(
        Value::Object(meta),
        json!({ "ui": { "resourceUri": "ui://weather/dashboard", "visibility": ["model", "app"] } })
    );
}

#[test]
fn tool_ui_omits_visibility_by_default_and_never_emits_flat_key() {
    let meta = ToolUi::new("ui://weather/dashboard").unwrap().into_meta();

    assert_eq!(
        Value::Object(meta.clone()),
        json!({ "ui": { "resourceUri": "ui://weather/dashboard" } })
    );
    assert!(!meta.contains_key("ui/resourceUri"));
}

#[test]
fn tool_ui_visibility_only_has_no_resource_uri() {
    let ui = ToolUi::visibility_only([UiVisibility::Model]);

    assert_eq!(ui.resource_uri(), None);
    assert_eq!(
        Value::Object(ui.into_meta()),
        json!({ "ui": { "visibility": ["model"] } })
    );
}

#[test]
fn app_only_means_visibility_without_model() {
    let app_only = ToolUi::visibility_only([UiVisibility::App]).into_meta();
    let shared = ToolUi::visibility_only([UiVisibility::App, UiVisibility::Model]).into_meta();
    let default = ToolUi::new("ui://x").unwrap().into_meta();

    assert!(is_app_only(&app_only));
    assert!(!is_app_only(&shared));
    assert!(!is_app_only(&default));
    assert!(!is_app_only(&Map::new()));
}

#[test]
fn resource_meta_serializes_camel_case() {
    let meta = UiResourceMeta {
        csp: Some(UiCsp {
            connect_domains: Some(vec!["wss://stream.example.com".into()]),
            resource_domains: Some(vec!["https://cdn.example.com".into()]),
            frame_domains: Some(vec!["https://embed.example.com".into()]),
            base_uri_domains: Some(vec!["https://example.com".into()]),
        }),
        permissions: Some(UiPermissions {
            camera: Some(EmptyObject {}),
            clipboard_write: Some(EmptyObject {}),
            ..Default::default()
        }),
        domain: Some("weather.example.com".into()),
        prefers_border: Some(true),
    };
    let expected = serde_json::to_value(&meta).unwrap();
    let domain = meta.domain.as_ref().unwrap().as_ptr();
    let meta = meta.into_meta().unwrap();

    assert_eq!(meta["ui"], expected);
    assert_eq!(meta["ui"]["domain"].as_str().unwrap().as_ptr(), domain);
    assert_eq!(
        Value::Object(meta),
        json!({
            "ui": {
                "csp": {
                    "connectDomains": ["wss://stream.example.com"],
                    "resourceDomains": ["https://cdn.example.com"],
                    "frameDomains": ["https://embed.example.com"],
                    "baseUriDomains": ["https://example.com"]
                },
                "permissions": { "camera": {}, "clipboardWrite": {} },
                "domain": "weather.example.com",
                "prefersBorder": true
            }
        })
    );
}

#[test]
fn csp_accepts_websockets_for_connect_only() {
    for origin in [
        "ws://localhost:8080",
        "http://api.local",
        "https://[::1]:8443/x",
        "https://*.example.com",
    ] {
        assert!(csp_meta(origin).is_ok(), "{origin:?}");
    }

    let resource = UiCsp {
        resource_domains: Some(vec!["wss://cdn.example.com".into()]),
        ..Default::default()
    };
    assert_eq!(
        resource.validate(),
        Err(AppsError::InvalidOrigin {
            directive: "resourceDomains",
            expected: "http(s)://",
            origin: "wss://cdn.example.com".into(),
        })
    );
}

#[test]
fn csp_rejects_malformed_origins() {
    for origin in [
        "api.example.com",
        "ftp://files.example.com",
        "https://",
        "https:///x",
        "https://evil.com\" onload",
        "https://a b.com",
        "https://a.com;script-src *",
        "https://a.com,https://b.com",
        "https://a\0b.com",
        "https://\u{a0}a.com",
    ] {
        assert_matches!(
            csp_meta(origin),
            Err(AppsError::InvalidOrigin { .. }),
            "{origin:?}"
        );
    }
}

#[test]
fn client_supports_mcp_apps_when_mime_type_advertised() {
    let caps = client_caps(json!({
        "extensions": { "io.modelcontextprotocol/ui": { "mimeTypes": [MIME_TYPE] } }
    }));

    assert!(caps.supports_mcp_apps());
    assert_eq!(
        caps.extension(EXTENSION_ID),
        Some(&json!({ "mimeTypes": [MIME_TYPE] }))
    );
}

#[test]
fn client_does_not_support_mcp_apps_without_matching_settings() {
    for caps in [
        json!({}),
        json!({ "extensions": {} }),
        json!({ "extensions": { "io.modelcontextprotocol/ui": {} } }),
        json!({ "extensions": { "io.modelcontextprotocol/ui": { "mimeTypes": ["text/html"] } } }),
        json!({ "extensions": { "io.modelcontextprotocol/ui": { "mimeTypes": MIME_TYPE } } }),
        json!({ "extensions": { "io.modelcontextprotocol/ui": { "mimeTypes": [1, null] } } }),
        json!({ "experimental": { "io.modelcontextprotocol/ui": { "mimeTypes": [MIME_TYPE] } } }),
    ] {
        assert!(!client_caps(caps.clone()).supports_mcp_apps(), "{caps}");
    }
}
