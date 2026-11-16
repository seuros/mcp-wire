use serde_json::Value;

/// Whether `value` decodes as `T`, through both the text and the `Value`
/// deserializer, which reach visitors through different string callbacks.
/// Panics if the two disagree.
pub(crate) fn decodes<T: serde::de::DeserializeOwned>(value: &Value) -> bool {
    let from_text = serde_json::from_str::<T>(&value.to_string()).is_ok();
    let from_value = serde_json::from_value::<T>(value.clone()).is_ok();
    assert_eq!(from_text, from_value, "deserializers disagree on {value}");
    from_value
}

/// A priority from a literal inside `0.0..=1.0`.
pub(crate) fn priority(value: f64) -> crate::content::annotations::Priority {
    value.try_into().expect("priority between 0 and 1")
}
