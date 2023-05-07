use serde_json::Value;

pub fn merge_json(own: &mut Value, other: &Value) {
    match (own, other) {
        (&mut Value::Object(ref mut own), Value::Object(other)) => {
            for (key, value) in other {
                merge_json(own.entry(key.clone()).or_insert(Value::Null), value);
            }
        }
        (own, other) => {
            *own = other.clone();
        }
    }
}

pub fn prune_equal_branches(own: &mut Value, other: &Value) {
    if own == other {
        *own = Value::Object(Default::default());
        return;
    }
    if let (&mut Value::Object(ref mut own), Value::Object(ref other)) = (own, other) {
        let mut keys_to_remove = vec![];
        for (key, own_value) in own.iter_mut() {
            if let Some(other_value) = other.get(key) {
                if own_value == other_value {
                    keys_to_remove.push(key.clone());
                    continue;
                }
                prune_equal_branches(own_value, other_value);
            }
        }
        for key in keys_to_remove {
            own.remove(&key);
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::from_str;

    use super::*;

    #[test]
    fn empty_value_is_set_to_an_object() {
        let mut own = Value::Null;
        let other = Value::Null;

        prune_equal_branches(&mut own, &other);

        assert_eq!(own, Value::Object(Default::default()));
    }

    #[test]
    fn different_types_are_kept() {
        let mut own: Value = from_str(r#"{"a":42,"b":true,"c":null}"#).unwrap();
        let original_own = own.clone();
        let other: Value = from_str(r#"{"a":true,"b":null,"c":42}"#).unwrap();

        prune_equal_branches(&mut own, &other);

        assert_eq!(own, original_own);
    }

    #[test]
    fn only_deep_leafs_are_kept() {
        let mut own: Value = from_str(r#"{"a":{"b":{"c":42},"d":{"e":1337}}}"#).unwrap();
        let other: Value = from_str(r#"{"a":{"b":{"c":true},"d":{"e":1337}}}"#).unwrap();

        prune_equal_branches(&mut own, &other);

        assert_eq!(own, from_str::<Value>(r#"{"a":{"b":{"c":42}}}"#).unwrap());
    }
}
