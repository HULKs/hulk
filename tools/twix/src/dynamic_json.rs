use ros_z::dynamic::{DynamicPayload, DynamicStruct, DynamicValue, EnumPayloadValue, EnumValue};
use serde_json::{Map, Number, Value};

pub fn dynamic_payload_to_json(payload: &DynamicPayload) -> Value {
    match &payload.value {
        DynamicValue::Struct(message) => dynamic_message_to_json(message),
        value => dynamic_value_to_json(value),
    }
}

pub fn dynamic_message_to_json(message: &DynamicStruct) -> Value {
    let mut fields = Map::new();
    for (name, value) in message.iter() {
        fields.insert(name.to_string(), dynamic_value_to_json(value));
    }
    Value::Object(fields)
}

fn dynamic_value_to_json(value: &DynamicValue) -> Value {
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
        DynamicValue::Float32(value) => Number::from_f64(*value as f64)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        DynamicValue::Float64(value) => Number::from_f64(*value)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        DynamicValue::String(value) => Value::String(value.clone()),
        DynamicValue::Bytes(value) => Value::Array(
            value
                .iter()
                .map(|byte| Value::Number((*byte).into()))
                .collect(),
        ),
        DynamicValue::Struct(value) => dynamic_message_to_json(value),
        DynamicValue::Optional(None) => Value::Null,
        DynamicValue::Optional(Some(value)) => dynamic_value_to_json(value),
        DynamicValue::Enum(value) => enum_value_to_json(value),
        DynamicValue::Sequence(values) => {
            Value::Array(values.iter().map(dynamic_value_to_json).collect())
        }
        DynamicValue::Map(entries) => Value::Array(
            entries
                .iter()
                .map(|(key, value)| {
                    let mut entry = Map::new();
                    entry.insert("key".to_string(), dynamic_value_to_json(key));
                    entry.insert("value".to_string(), dynamic_value_to_json(value));
                    Value::Object(entry)
                })
                .collect(),
        ),
    }
}

fn enum_value_to_json(value: &EnumValue) -> Value {
    let mut fields = Map::new();
    fields.insert(
        "variant_index".to_string(),
        Value::Number(value.variant_index.into()),
    );
    fields.insert(
        "variant_name".to_string(),
        Value::String(value.variant_name.clone()),
    );
    fields.insert("payload".to_string(), enum_payload_to_json(&value.payload));
    Value::Object(fields)
}

fn enum_payload_to_json(payload: &EnumPayloadValue) -> Value {
    match payload {
        EnumPayloadValue::Unit => Value::Null,
        EnumPayloadValue::Newtype(value) => dynamic_value_to_json(value),
        EnumPayloadValue::Tuple(values) => {
            Value::Array(values.iter().map(dynamic_value_to_json).collect())
        }
        EnumPayloadValue::Struct(fields) => Value::Object(
            fields
                .iter()
                .map(|field| (field.name.clone(), dynamic_value_to_json(&field.value)))
                .collect(),
        ),
    }
}
