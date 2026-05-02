use std::time::Duration;

use color_eyre::eyre::{Result, WrapErr, bail, eyre};
use ros_z::dynamic::{
    DynamicNamedValue, DynamicPayload, DynamicStruct, DynamicSubscriber, DynamicValue,
    EnumPayloadValue, EnumValue,
};
use serde_json::{Map, Value};

use crate::{
    app::AppContext,
    model::echo::{EchoHeader, EchoMessageView},
    render::{OutputMode, json, text},
};

const TYPE_DISCOVERY_TIMEOUT: Duration = Duration::from_secs(5);

pub async fn run(
    app: &AppContext,
    output_mode: OutputMode,
    topic: &str,
    count: Option<usize>,
    timeout: Option<f64>,
) -> Result<()> {
    let subscriber_builder = app
        .create_dynamic_subscriber_builder(topic, TYPE_DISCOVERY_TIMEOUT)
        .await
        .wrap_err_with(|| format!("failed to subscribe to {topic}"))?;
    let subscriber = subscriber_builder
        .build()
        .await
        .map_err(|error| eyre!(error))
        .wrap_err_with(|| format!("failed to subscribe to {topic}"))?;
    let _schema = subscriber
        .schema()
        .ok_or_else(|| eyre!("dynamic subscriber missing schema for {topic}"))?;
    let type_name = subscriber
        .entity()
        .type_info
        .as_ref()
        .map(|type_info| type_info.name.clone())
        .unwrap_or_else(|| "unknown".to_string());
    let header = EchoHeader::new(
        topic.to_string(),
        type_name.clone(),
        display_schema_hash(&subscriber),
    );
    let deadline = timeout
        .map(Duration::from_secs_f64)
        .map(|timeout| tokio::time::Instant::now() + timeout);
    let mut seen = 0usize;

    if output_mode.is_text() {
        text::print_echo_header(&header);
    }

    loop {
        if count.is_some_and(|limit| seen >= limit) {
            return Ok(());
        }

        let message = receive_message(&subscriber, deadline, topic).await?;
        seen += 1;

        match output_mode {
            OutputMode::Json => {
                let view = EchoMessageView::new(
                    header.topic.clone(),
                    type_name.clone(),
                    header.schema_hash.clone(),
                    dynamic_payload_to_json(&message),
                );
                json::print_line(&view)?;
            }
            OutputMode::Text => {
                text::print_echo_message(&format_payload_pretty(&message), count, seen);
            }
        }
    }
}

fn display_schema_hash(subscriber: &DynamicSubscriber) -> String {
    display_schema_hash_from_type_info(subscriber.entity().type_info.as_ref())
}

fn display_schema_hash_from_type_info(type_info: Option<&ros_z::TypeInfo>) -> String {
    type_info
        .and_then(|type_info| type_info.hash)
        .map(|hash| hash.to_hash_string())
        .unwrap_or_else(|| "unknown".to_string())
}

async fn receive_message(
    subscriber: &DynamicSubscriber,
    deadline: Option<tokio::time::Instant>,
    topic: &str,
) -> Result<DynamicPayload> {
    let receive = subscriber.recv();

    match deadline {
        Some(deadline) => match tokio::time::timeout_at(deadline, receive).await {
            Ok(result) => {
                result.map_err(|error| eyre!("subscriber receive failed for {topic}: {error}"))
            }
            Err(_) => bail!("timed out waiting for messages on {topic}"),
        },
        None => receive
            .await
            .map_err(|error| eyre!("subscriber receive failed for {topic}: {error}")),
    }
}

fn dynamic_payload_to_json(payload: &DynamicPayload) -> Value {
    dynamic_value_to_json(&payload.value)
}

fn dynamic_message_to_json(message: &DynamicStruct) -> Value {
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
        DynamicValue::Float32(value) => serde_json::Number::from_f64(*value as f64)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        DynamicValue::Float64(value) => serde_json::Number::from_f64(*value)
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

fn format_message_pretty(message: &DynamicStruct) -> String {
    let mut output = String::new();
    for (name, value) in message.iter() {
        format_value_pretty(&mut output, name, value, 0);
    }
    output
}

fn format_payload_pretty(payload: &DynamicPayload) -> String {
    match &payload.value {
        DynamicValue::Struct(message) => format_message_pretty(message),
        value => {
            let mut output = String::new();
            format_value_pretty(&mut output, "value", value, 0);
            output
        }
    }
}

fn format_value_pretty(output: &mut String, name: &str, value: &DynamicValue, indent: usize) {
    let prefix = "  ".repeat(indent);

    match value {
        DynamicValue::Bool(value) => output.push_str(&format!("{}{}: {}\n", prefix, name, value)),
        DynamicValue::Int8(value) => output.push_str(&format!("{}{}: {}\n", prefix, name, value)),
        DynamicValue::Int16(value) => output.push_str(&format!("{}{}: {}\n", prefix, name, value)),
        DynamicValue::Int32(value) => output.push_str(&format!("{}{}: {}\n", prefix, name, value)),
        DynamicValue::Int64(value) => output.push_str(&format!("{}{}: {}\n", prefix, name, value)),
        DynamicValue::Uint8(value) => output.push_str(&format!("{}{}: {}\n", prefix, name, value)),
        DynamicValue::Uint16(value) => output.push_str(&format!("{}{}: {}\n", prefix, name, value)),
        DynamicValue::Uint32(value) => output.push_str(&format!("{}{}: {}\n", prefix, name, value)),
        DynamicValue::Uint64(value) => output.push_str(&format!("{}{}: {}\n", prefix, name, value)),
        DynamicValue::Float32(value) => {
            output.push_str(&format!("{}{}: {}\n", prefix, name, value))
        }
        DynamicValue::Float64(value) => {
            output.push_str(&format!("{}{}: {}\n", prefix, name, value))
        }
        DynamicValue::String(value) => {
            output.push_str(&format!("{}{}: \"{}\"\n", prefix, name, value))
        }
        DynamicValue::Bytes(value) => output.push_str(&format!(
            "{}{}: [bytes: {} bytes]\n",
            prefix,
            name,
            value.len()
        )),
        DynamicValue::Struct(value) => {
            output.push_str(&format!("{}{}:\n", prefix, name));
            for (nested_name, nested_value) in value.iter() {
                format_value_pretty(output, nested_name, nested_value, indent + 1);
            }
        }
        DynamicValue::Optional(None) => output.push_str(&format!("{}{}: null\n", prefix, name)),
        DynamicValue::Optional(Some(value)) => format_value_pretty(output, name, value, indent),
        DynamicValue::Enum(value) => format_enum_value_pretty(output, name, value, indent),
        DynamicValue::Sequence(values) => {
            if values.is_empty() {
                output.push_str(&format!("{}{}[]: []\n", prefix, name));
            } else {
                output.push_str(&format!("{}{}[{}]:\n", prefix, name, values.len()));
                for (index, value) in values.iter().enumerate() {
                    format_value_pretty(output, &format!("[{}]", index), value, indent + 1);
                }
            }
        }
        DynamicValue::Map(entries) => {
            if entries.is_empty() {
                output.push_str(&format!("{}{}: {{}}\n", prefix, name));
            } else {
                output.push_str(&format!("{}{}[{}]:\n", prefix, name, entries.len()));
                for (key, value) in entries {
                    format_value_pretty(output, "key", key, indent + 1);
                    format_value_pretty(output, "value", value, indent + 1);
                }
            }
        }
    }
}

fn format_enum_value_pretty(output: &mut String, name: &str, value: &EnumValue, indent: usize) {
    let prefix = "  ".repeat(indent);
    output.push_str(&format!("{}{}:\n", prefix, name));
    output.push_str(&format!("{}  variant: {}\n", prefix, value.variant_name));
    format_enum_payload_pretty(output, &value.payload, indent + 1);
}

fn format_enum_payload_pretty(output: &mut String, payload: &EnumPayloadValue, indent: usize) {
    let prefix = "  ".repeat(indent);

    match payload {
        EnumPayloadValue::Unit => output.push_str(&format!("{}payload: null\n", prefix)),
        EnumPayloadValue::Newtype(value) => format_value_pretty(output, "payload", value, indent),
        EnumPayloadValue::Tuple(values) => {
            if values.is_empty() {
                output.push_str(&format!("{}payload[]: []\n", prefix));
            } else {
                output.push_str(&format!("{}payload[{}]:\n", prefix, values.len()));
                for (index, value) in values.iter().enumerate() {
                    format_value_pretty(output, &format!("[{}]", index), value, indent + 1);
                }
            }
        }
        EnumPayloadValue::Struct(fields) => {
            if fields.is_empty() {
                output.push_str(&format!("{}payload: {{}}\n", prefix));
            } else {
                output.push_str(&format!("{}payload:\n", prefix));
                format_named_fields_pretty(output, fields, indent + 1);
            }
        }
    }
}

fn format_named_fields_pretty(output: &mut String, fields: &[DynamicNamedValue], indent: usize) {
    for field in fields {
        format_value_pretty(output, &field.name, &field.value, indent);
    }
}

#[cfg(test)]
mod tests {
    use ros_z::{SchemaHash, TypeInfo};

    use super::display_schema_hash_from_type_info;

    #[test]
    fn display_schema_hash_prefers_advertised_canonical_hash() {
        let hash = SchemaHash::from_hash_string(
            "RZHS01_1111111111111111111111111111111111111111111111111111111111111111",
        )
        .expect("hash");
        let type_info = TypeInfo::with_hash("std_msgs::String", hash);

        assert_eq!(
            display_schema_hash_from_type_info(Some(&type_info)),
            "RZHS01_1111111111111111111111111111111111111111111111111111111111111111"
        );
    }

    #[test]
    fn display_schema_hash_reports_unknown_without_advertised_hash() {
        let type_info = TypeInfo::new("std_msgs::String", None);

        assert_eq!(
            display_schema_hash_from_type_info(Some(&type_info)),
            "unknown"
        );
    }
}
