use std::collections::BTreeSet;

use serde_json::{Map, Value};

use super::{LayerPath, ParameterError, ProvenanceMap, Result};

#[derive(Debug, Clone)]
pub(crate) struct RecursiveDiffEntry {
    pub path: String,
    pub old_value: Value,
    pub new_value: Value,
}

pub(crate) type RecursiveDiff = Vec<RecursiveDiffEntry>;

#[derive(Debug, Clone)]
pub(crate) struct MergedParameters {
    pub effective: Value,
    pub provenance: ProvenanceMap,
}

pub(crate) fn merge_layers(layers: &[(&str, &Value)]) -> Result<MergedParameters> {
    let mut provenance = ProvenanceMap::new();
    let mut effective = Value::Object(Map::new());

    for (layer, overlay) in layers {
        effective = merge_value_into("", effective, overlay, layer, &mut provenance);
    }

    Ok(MergedParameters {
        effective,
        provenance,
    })
}

fn merge_value_into(
    path: &str,
    base: Value,
    overlay: &Value,
    layer: &str,
    provenance: &mut ProvenanceMap,
) -> Value {
    match (base, overlay) {
        (Value::Object(mut left), Value::Object(right)) => {
            if !path.is_empty() {
                provenance.insert(path.to_string(), layer.to_string());
            }

            for key in right.keys() {
                let child_path = join_path(path, key);
                let merged = match left.remove(key) {
                    Some(left_value) => {
                        merge_value_into(&child_path, left_value, &right[key], layer, provenance)
                    }
                    None => merge_value(&child_path, &right[key], Some(layer), provenance),
                };
                left.insert(key.clone(), merged);
            }

            Value::Object(left)
        }
        (_, right) => merge_value(path, right, Some(layer), provenance),
    }
}

fn merge_value(
    path: &str,
    value: &Value,
    layer: Option<&str>,
    provenance: &mut ProvenanceMap,
) -> Value {
    if let Some(layer) = layer
        && !path.is_empty()
    {
        provenance.insert(path.to_string(), layer.to_string());
    }

    match value {
        Value::Object(map) => {
            let mut merged = Map::new();
            for (key, child) in map {
                let child_path = join_path(path, key);
                merged.insert(
                    key.clone(),
                    merge_value(&child_path, child, layer, provenance),
                );
            }
            Value::Object(merged)
        }
        _ => value.clone(),
    }
}

pub(crate) fn get_value_at_path(root: &Value, path: &str) -> Result<Option<Value>> {
    let segments = parse_path(path)?;
    let mut current = root;

    for segment in segments {
        match current {
            Value::Object(map) => match map.get(segment) {
                Some(next) => current = next,
                None => return Ok(None),
            },
            _ => return Ok(None),
        }
    }

    Ok(Some(current.clone()))
}

pub(crate) fn set_value_at_path(root: &mut Value, path: &str, value: Value) -> Result<()> {
    let segments = parse_path(path)?;
    if !root.is_object() {
        if segments.len() > 1 {
            return Err(ParameterError::PathError {
                path: path.to_string(),
                reason: "encountered non-object at root while creating path".to_string(),
            });
        }
        *root = Value::Object(Map::new());
    }

    let mut current = root;
    for segment in &segments[..segments.len() - 1] {
        let map = current
            .as_object_mut()
            .ok_or_else(|| ParameterError::PathError {
                path: path.to_string(),
                reason: "encountered non-object while creating path".to_string(),
            })?;
        current = map
            .entry((*segment).to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        if !current.is_object() {
            return Err(ParameterError::PathError {
                path: path.to_string(),
                reason: format!("encountered non-object at '{segment}' while creating path"),
            });
        }
    }

    let last = segments
        .last()
        .expect("validated path has at least one segment");
    current
        .as_object_mut()
        .expect("path parent is object")
        .insert((*last).to_string(), value);

    Ok(())
}

pub(crate) fn remove_value_at_path(root: &mut Value, path: &str) -> Result<bool> {
    let segments = parse_path(path)?;
    let mut current = root;
    for segment in &segments[..segments.len() - 1] {
        let Some(next) = current
            .as_object_mut()
            .and_then(|map| map.get_mut(*segment))
        else {
            return Ok(false);
        };
        current = next;
    }

    let last = segments
        .last()
        .expect("validated path has at least one segment");
    Ok(current
        .as_object_mut()
        .and_then(|map| map.remove(*last))
        .is_some())
}

pub(crate) fn recursive_diff(old: &Value, new: &Value) -> RecursiveDiff {
    let mut out = Vec::new();
    diff_value("", old, new, &mut out);
    out
}

fn diff_value(path: &str, old: &Value, new: &Value, out: &mut RecursiveDiff) {
    match (old, new) {
        (Value::Object(left), Value::Object(right)) => {
            let keys: BTreeSet<_> = left.keys().chain(right.keys()).cloned().collect();
            for key in keys {
                let child_path = join_path(path, &key);
                match (left.get(&key), right.get(&key)) {
                    (Some(l), Some(r)) => diff_value(&child_path, l, r, out),
                    (Some(l), None) => out.push(RecursiveDiffEntry {
                        path: child_path,
                        old_value: l.clone(),
                        new_value: Value::Null,
                    }),
                    (None, Some(r)) => out.push(RecursiveDiffEntry {
                        path: child_path,
                        old_value: Value::Null,
                        new_value: r.clone(),
                    }),
                    (None, None) => {}
                }
            }
        }
        _ if old != new => out.push(RecursiveDiffEntry {
            path: path.to_string(),
            old_value: old.clone(),
            new_value: new.clone(),
        }),
        _ => {}
    }
}

pub(crate) fn provenance_for_path(provenance: &ProvenanceMap, path: &str) -> Option<LayerPath> {
    provenance.get(path).cloned()
}

fn parse_path(path: &str) -> Result<Vec<&str>> {
    if path.is_empty() {
        return Err(ParameterError::PathError {
            path: path.to_string(),
            reason: "path must not be empty".to_string(),
        });
    }

    let segments: Vec<_> = path.split('.').collect();
    if segments.iter().any(|segment| segment.is_empty()) {
        return Err(ParameterError::PathError {
            path: path.to_string(),
            reason: "path segments must not be empty".to_string(),
        });
    }
    if path.contains('[') || path.contains(']') {
        return Err(ParameterError::PathError {
            path: path.to_string(),
            reason: "array indexing is not supported".to_string(),
        });
    }

    Ok(segments)
}

fn join_path(prefix: &str, segment: &str) -> String {
    if prefix.is_empty() {
        segment.to_string()
    } else {
        format!("{prefix}.{segment}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn merge_prefers_higher_precedence_scalars() {
        let base = json!({"a": 1, "nested": {"x": 1, "y": 2}});
        let location = json!({"nested": {"x": 9}});
        let robot = json!({"a": 5});
        let merged = merge_layers(&[
            ("./parameters/base", &base),
            ("./parameters/location", &location),
            ("./parameters/robot", &robot),
        ])
        .unwrap();

        assert_eq!(
            merged.effective,
            json!({"a": 5, "nested": {"x": 9, "y": 2}})
        );
        assert_eq!(
            merged.provenance.get("a"),
            Some(&"./parameters/robot".to_string())
        );
        assert_eq!(
            merged.provenance.get("nested.x"),
            Some(&"./parameters/location".to_string())
        );
        assert_eq!(
            merged.provenance.get("nested.y"),
            Some(&"./parameters/base".to_string())
        );
    }

    #[test]
    fn set_get_remove_round_trip() {
        let mut root = json!({});
        set_value_at_path(&mut root, "vision.ball.threshold", json!(0.7)).unwrap();
        assert_eq!(
            get_value_at_path(&root, "vision.ball.threshold").unwrap(),
            Some(json!(0.7))
        );
        assert!(remove_value_at_path(&mut root, "vision.ball.threshold").unwrap());
        assert_eq!(
            get_value_at_path(&root, "vision.ball.threshold").unwrap(),
            None
        );
    }

    #[test]
    fn set_value_at_path_rejects_traversal_through_scalar() {
        let mut value = serde_json::json!({ "a": 1 });
        let error = set_value_at_path(&mut value, "a.b", serde_json::json!(2))
            .expect_err("scalar traversal should be rejected");
        assert!(error.to_string().contains("a"));
        assert_eq!(value, serde_json::json!({ "a": 1 }));
    }

    #[test]
    fn set_value_at_path_rejects_traversal_from_scalar_root() {
        let mut value = serde_json::json!(1);
        let error = set_value_at_path(&mut value, "a.b", serde_json::json!(2))
            .expect_err("scalar root traversal should be rejected");
        assert!(error.to_string().contains("root"));
        assert_eq!(value, serde_json::json!(1));
    }

    #[test]
    fn recursive_diff_reports_nested_changes() {
        let diff = recursive_diff(
            &json!({"a": 1, "nested": {"x": 1}}),
            &json!({"a": 2, "nested": {"x": 1, "y": true}}),
        );

        assert_eq!(diff.len(), 2);
        assert_eq!(diff[0].path, "a");
        assert_eq!(diff[1].path, "nested.y");
    }
}
