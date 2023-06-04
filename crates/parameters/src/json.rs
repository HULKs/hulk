use serde_json::{Map, Value};

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

pub fn copy_nested_value(value: &Value, path: &str) -> Option<Value> {
    if path.is_empty() {
        return Some(value.clone());
    }
    let (prefix, suffix) = match path.split_once('.') {
        Some(parts) => parts,
        None => (path, ""),
    };
    match value {
        Value::Object(object) => {
            let nested_value = object.get(prefix)?;
            let nested_copied_value = copy_nested_value(nested_value, suffix)?;
            Some(Value::Object(Map::from_iter([(
                prefix.to_string(),
                nested_copied_value,
            )])))
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) | Value::Array(_) => {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

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
        let mut own = json!({"a":42,"b":true,"c":null});
        let original_own = own.clone();
        let other = json!({"a":true,"b":null,"c":42});

        prune_equal_branches(&mut own, &other);

        assert_eq!(own, original_own);
    }

    #[test]
    fn only_deep_leafs_are_kept() {
        let mut own = json!({"a":{"b":{"c":42},"d":{"e":1337}}});
        let other = json!({"a":{"b":{"c":true},"d":{"e":1337}}});

        prune_equal_branches(&mut own, &other);

        assert_eq!(own, json!({"a":{"b":{"c":42}}}));
    }

    #[test]
    fn branches_matching_the_path_are_retained_others_are_removed() {
        let value = json!({"a":{"b":{"c":42},"d":{"e":1337}}});

        let copied = copy_nested_value(&value, "a.b.c");

        assert_eq!(copied, Some(json!({"a":{"b":{"c":42}}})));
    }

    #[test]
    fn branches_matching_parts_of_the_path_are_retained_others_are_removed() {
        let value = json!({"a":{"b":{"c":42},"d":{"e":1337}}});

        let copied = copy_nested_value(&value, "a.b");

        assert_eq!(copied, Some(json!({"a":{"b":{"c":42}}})));
    }

    #[test]
    fn all_branches_are_removed_for_non_existant_path() {
        let value = json!({"a":{"b":{"c":42},"d":{"e":1337}}});

        let copied = copy_nested_value(&value, "not.matching");

        assert_eq!(copied, None);
    }

    #[test]
    fn all_branches_are_removed_for_too_long_path() {
        let value = json!({"a":{"b":{"c":42},"d":{"e":1337}}});

        let copied = copy_nested_value(&value, "a.b.c.too.long");

        assert_eq!(copied, None);
    }

    #[test]
    fn all_branches_are_retained_for_non_empty_path() {
        let value = json!({"a":{"b":{"c":42},"d":{"e":1337}}});

        let copied = copy_nested_value(&value, "");

        assert_eq!(copied, Some(value));
    }
}
