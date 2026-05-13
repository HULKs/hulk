use ros_z::dynamic::{DynamicPayload, DynamicStruct, DynamicValue, EnumPayloadValue, EnumValue};
use serde_json::{Map, Number, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JsonRenderPolicy {
    pub bytes: ByteRenderPolicy,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ByteRenderPolicy {
    Compact { preview_len: usize },
    FullArray,
}

impl Default for JsonRenderPolicy {
    fn default() -> Self {
        Self {
            bytes: ByteRenderPolicy::Compact { preview_len: 32 },
        }
    }
}

pub fn dynamic_payload_to_json(payload: &DynamicPayload, policy: JsonRenderPolicy) -> Value {
    dynamic_value_to_json(&payload.value, policy)
}

pub fn dynamic_value_to_json(value: &DynamicValue, policy: JsonRenderPolicy) -> Value {
    match value {
        DynamicValue::Bool(value) => Value::Bool(*value),
        DynamicValue::Int8(value) => Value::Number((*value).into()),
        DynamicValue::Int16(value) => Value::Number((*value).into()),
        DynamicValue::Int32(value) => Value::Number((*value).into()),
        DynamicValue::Int64(value) => Value::Number((*value).into()),
        DynamicValue::Uint8(value) => Value::Number((*value).into()),
        DynamicValue::Uint16(value) => Value::Number((*value).into()),
        DynamicValue::Uint32(value) => Value::Number((*value).into()),
        DynamicValue::Uint64(value) => Value::Number((*value).into()),
        DynamicValue::Float32(value) => Number::from_f64(f64::from(*value))
            .map(Value::Number)
            .unwrap_or(Value::Null),
        DynamicValue::Float64(value) => Number::from_f64(*value)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        DynamicValue::String(value) => Value::String(value.clone()),
        DynamicValue::Bytes(bytes) => bytes_to_json(bytes, policy.bytes),
        DynamicValue::Struct(value) => struct_to_json(value, policy),
        DynamicValue::Optional(None) => Value::Null,
        DynamicValue::Optional(Some(value)) => dynamic_value_to_json(value, policy),
        DynamicValue::Enum(value) => enum_to_json(value, policy),
        DynamicValue::Sequence(values) => Value::Array(
            values
                .iter()
                .map(|value| dynamic_value_to_json(value, policy))
                .collect(),
        ),
        DynamicValue::Map(entries) => Value::Array(
            entries
                .iter()
                .map(|(key, value)| {
                    let mut entry = Map::new();
                    entry.insert("key".to_string(), dynamic_value_to_json(key, policy));
                    entry.insert("value".to_string(), dynamic_value_to_json(value, policy));
                    Value::Object(entry)
                })
                .collect(),
        ),
    }
}

fn bytes_to_json(bytes: &[u8], policy: ByteRenderPolicy) -> Value {
    match policy {
        ByteRenderPolicy::Compact { preview_len } => {
            let mut fields = Map::new();
            fields.insert("$type".to_string(), Value::String("bytes".to_string()));
            fields.insert("len".to_string(), Value::from(bytes.len()));
            fields.insert(
                "preview".to_string(),
                Value::Array(
                    bytes
                        .iter()
                        .take(preview_len)
                        .map(|byte| Value::from(*byte))
                        .collect(),
                ),
            );
            fields.insert(
                "truncated".to_string(),
                Value::Bool(bytes.len() > preview_len),
            );
            Value::Object(fields)
        }
        ByteRenderPolicy::FullArray => {
            Value::Array(bytes.iter().map(|byte| Value::from(*byte)).collect())
        }
    }
}

fn struct_to_json(value: &DynamicStruct, policy: JsonRenderPolicy) -> Value {
    Value::Object(
        value
            .iter()
            .map(|(name, value)| (name.to_string(), dynamic_value_to_json(value, policy)))
            .collect(),
    )
}

fn enum_to_json(value: &EnumValue, policy: JsonRenderPolicy) -> Value {
    let mut fields = Map::new();
    fields.insert(
        "variant_index".to_string(),
        Value::Number(value.variant_index.into()),
    );
    fields.insert(
        "variant_name".to_string(),
        Value::String(value.variant_name.clone()),
    );
    fields.insert(
        "payload".to_string(),
        enum_payload_to_json(&value.payload, policy),
    );
    Value::Object(fields)
}

fn enum_payload_to_json(payload: &EnumPayloadValue, policy: JsonRenderPolicy) -> Value {
    match payload {
        EnumPayloadValue::Unit => Value::Null,
        EnumPayloadValue::Newtype(value) => dynamic_value_to_json(value, policy),
        EnumPayloadValue::Tuple(values) => Value::Array(
            values
                .iter()
                .map(|value| dynamic_value_to_json(value, policy))
                .collect(),
        ),
        EnumPayloadValue::Struct(fields) => Value::Object(
            fields
                .iter()
                .map(|field| {
                    (
                        field.name.clone(),
                        dynamic_value_to_json(&field.value, policy),
                    )
                })
                .collect(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{ByteRenderPolicy, JsonRenderPolicy, dynamic_value_to_json};
    use ros_z::dynamic::DynamicValue;

    #[test]
    fn compact_bytes_include_len_preview_and_truncation() {
        let value = dynamic_value_to_json(
            &DynamicValue::Bytes(vec![0, 1, 2, 3]),
            JsonRenderPolicy {
                bytes: ByteRenderPolicy::Compact { preview_len: 2 },
            },
        );

        assert_eq!(
            value,
            serde_json::json!({
                "$type": "bytes",
                "len": 4,
                "preview": [0, 1],
                "truncated": true,
            })
        );
    }

    #[test]
    fn full_bytes_render_as_array() {
        let value = dynamic_value_to_json(
            &DynamicValue::Bytes(vec![0, 1, 2]),
            JsonRenderPolicy {
                bytes: ByteRenderPolicy::FullArray,
            },
        );

        assert_eq!(value, serde_json::json!([0, 1, 2]));
    }

    #[test]
    fn primitive_string_renders_as_json_string() {
        let value = dynamic_value_to_json(
            &DynamicValue::String("camera".to_string()),
            JsonRenderPolicy::default(),
        );

        assert_eq!(value, serde_json::json!("camera"));
    }

    #[test]
    fn sequence_renders_as_json_array() {
        let value = dynamic_value_to_json(
            &DynamicValue::Sequence(vec![DynamicValue::Int32(-7), DynamicValue::Uint32(9)]),
            JsonRenderPolicy::default(),
        );

        assert_eq!(value, serde_json::json!([-7, 9]));
    }
}
