use serde_json::{Map, Number, Value};

use super::{DynamicPayload, DynamicStruct, DynamicValue, EnumPayloadValue, EnumValue};

/// Rendering options for dynamic payload JSON values.
///
/// Primitive values render as their matching JSON scalar. Structs render as JSON
/// objects, sequences as arrays, maps as arrays of `{ "key", "value" }` entries,
/// optional `None` as `null`, and enums as objects with `variant_index`,
/// `variant_name`, and `payload` fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DynamicJsonRenderPolicy {
    /// Rendering policy for `DynamicValue::Bytes`.
    pub bytes: ByteRenderPolicy,
    /// Rendering policy for `NaN` and infinite floating point values.
    pub non_finite_float: NonFiniteFloatRenderPolicy,
}

impl Default for DynamicJsonRenderPolicy {
    fn default() -> Self {
        Self {
            bytes: ByteRenderPolicy::Compact { preview_len: 32 },
            non_finite_float: NonFiniteFloatRenderPolicy::Object,
        }
    }
}

/// JSON representation for dynamic byte arrays.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ByteRenderPolicy {
    /// Render bytes as `{ "$type": "bytes", "len", "preview", "truncated" }`.
    Compact { preview_len: usize },
    /// Render bytes as a JSON array of byte values.
    FullArray,
}

/// JSON representation for non-finite floating point values.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NonFiniteFloatRenderPolicy {
    /// Render non-finite floats as JSON `null`.
    Null,
    /// Render non-finite floats as `{ "$type": "non_finite_float", "value" }`.
    Object,
}

/// Render a dynamic payload value as JSON according to `policy`.
pub fn dynamic_payload_to_json(payload: &DynamicPayload, policy: DynamicJsonRenderPolicy) -> Value {
    dynamic_value_to_json(&payload.value, policy)
}

/// Render a dynamic value as JSON according to `policy`.
pub fn dynamic_value_to_json(value: &DynamicValue, policy: DynamicJsonRenderPolicy) -> Value {
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
        DynamicValue::Float32(value) => float_to_json(f64::from(*value), policy),
        DynamicValue::Float64(value) => float_to_json(*value, policy),
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

fn float_to_json(value: f64, policy: DynamicJsonRenderPolicy) -> Value {
    Number::from_f64(value)
        .map(Value::Number)
        .unwrap_or_else(|| non_finite_float_to_json(value, policy.non_finite_float))
}

fn non_finite_float_to_json(value: f64, policy: NonFiniteFloatRenderPolicy) -> Value {
    match policy {
        NonFiniteFloatRenderPolicy::Null => Value::Null,
        NonFiniteFloatRenderPolicy::Object => {
            let mut fields = Map::new();
            fields.insert(
                "$type".to_string(),
                Value::String("non_finite_float".to_string()),
            );
            fields.insert(
                "value".to_string(),
                Value::String(non_finite_float_label(value).to_string()),
            );
            Value::Object(fields)
        }
    }
}

fn non_finite_float_label(value: f64) -> &'static str {
    if value.is_nan() {
        "NaN"
    } else if value.is_sign_negative() {
        "-Infinity"
    } else {
        "Infinity"
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

fn struct_to_json(value: &DynamicStruct, policy: DynamicJsonRenderPolicy) -> Value {
    Value::Object(
        value
            .iter()
            .map(|(name, value)| (name.to_string(), dynamic_value_to_json(value, policy)))
            .collect(),
    )
}

fn enum_to_json(value: &EnumValue, policy: DynamicJsonRenderPolicy) -> Value {
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

fn enum_payload_to_json(payload: &EnumPayloadValue, policy: DynamicJsonRenderPolicy) -> Value {
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
