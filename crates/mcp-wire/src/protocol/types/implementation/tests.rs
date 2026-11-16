use serde_json::json;

use super::*;

#[test]
fn implementation_icons_are_typed_icon_arrays() {
    let implementation = Implementation::new("host", "1.0.0")
        .with_icons(vec![Icon::new("https://example.com/icon.png").unwrap()]);
    let value = serde_json::to_value(implementation).unwrap();
    assert_eq!(value["icons"][0]["src"], "https://example.com/icon.png");

    assert!(
        serde_json::from_value::<Implementation>(json!({
            "name": "host",
            "version": "1.0.0",
            "icons": "https://example.com/icon.png"
        }))
        .is_err()
    );
}

#[test]
fn website_url_must_be_a_uri_and_a_received_one_is_dropped() {
    for (url, valid) in [
        ("https://example.com", true),
        ("http://example.com/docs?page=1#intro", true),
        ("urn:example:host", true),
        ("example.com", false),
        ("", false),
        ("https://exa mple.com", false),
    ] {
        let expected = valid.then(|| WebsiteUrl(url.into()));
        assert_eq!(
            url.parse::<WebsiteUrl>(),
            expected
                .clone()
                .ok_or_else(|| InvalidWebsiteUrl(url.into()))
        );
        assert_eq!(
            crate::test_support::decodes::<WebsiteUrl>(&json!(url)),
            valid
        );
        // A received Implementation keeps the peer and drops the bad URL.
        let wire = json!({"name": "host", "version": "1.0.0", "websiteUrl": url});
        let received: Implementation = serde_json::from_value(wire).unwrap();
        assert_eq!(received.website_url, expected, "{url:?}");
    }
}
