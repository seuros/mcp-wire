/// A JSON object from fixed fields; `Map::from_iter` would sort them through a temporary `Vec`.
pub(crate) fn json_object<const N: usize>(
    fields: [(String, serde_json::Value); N],
) -> serde_json::Map<String, serde_json::Value> {
    let mut object = serde_json::Map::new();
    object.extend(fields);
    object
}
