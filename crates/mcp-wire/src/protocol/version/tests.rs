use super::*;

#[test]
fn test_supported_version() {
    assert!(is_supported_protocol_version("2025-11-25"));
    assert!(is_supported_protocol_version("2025-06-18"));
    assert!(!is_supported_protocol_version("2024-01-01"));
}

#[test]
fn test_negotiate_supported_version() {
    assert_eq!(
        negotiate_protocol_version("2025-11-25"),
        ProtocolVersion::V2025_11_25
    );
    assert_eq!(
        negotiate_protocol_version("2025-06-18"),
        ProtocolVersion::V2025_06_18
    );
}

#[test]
fn test_negotiate_unsupported_version() {
    assert_eq!(
        negotiate_protocol_version("2024-01-01"),
        ProtocolVersion::V2025_11_25
    );
}

#[test]
fn versions_order_by_release_and_round_trip_their_names() {
    assert!(ProtocolVersion::V2025_06_18 < ProtocolVersion::V2025_11_25);
    assert_eq!(ProtocolVersion::LATEST, ProtocolVersion::V2025_11_25);
    assert_eq!(SUPPORTED_PROTOCOL_VERSIONS, ["2025-11-25", "2025-06-18"]);
    for version in ProtocolVersion::ALL {
        assert_eq!(ProtocolVersion::parse(version.as_str()), Some(version));
        assert_eq!(version.to_string(), version.as_str());
    }
    assert_eq!(ProtocolVersion::parse("2099-01-01"), None);
}
