use super::*;
use crate::test_support::priority;

#[test]
fn test_annotations_builder() {
    let ann = Annotations::new()
        .with_priority(priority(0.8))
        .add_audience(Role::User)
        .add_audience(Role::Assistant);

    assert_eq!(ann.priority, Some(priority(0.8)));
    assert_eq!(ann.audience, Some(vec![Role::User, Role::Assistant]));
}

#[test]
fn priority_accepts_only_zero_through_one() {
    for value in [0.0, -0.0, 0.5, 1.0] {
        assert_eq!(Priority::try_from(value).map(Priority::get), Ok(value));
    }
    for value in [-0.1, 1.1, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(Priority::try_from(value).is_err(), "{value}");
    }
}

#[test]
fn test_serialization() {
    let ann = Annotations::new().with_priority(priority(0.9));

    let json = serde_json::to_value(&ann).unwrap();
    assert_eq!(json["priority"], 0.9);
    assert!(json.get("audience").is_none());
    assert_eq!(ann.into_value(), json);
}

#[test]
fn test_priority_deserialization_rejects_invalid_values() {
    for priority in [
        serde_json::json!(-0.1),
        serde_json::json!(1.1),
        serde_json::Value::Null,
    ] {
        assert!(
            serde_json::from_value::<Annotations>(serde_json::json!({
                "priority": priority
            }))
            .is_err()
        );
    }
}
