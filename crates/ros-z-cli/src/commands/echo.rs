use std::time::Duration;

use color_eyre::eyre::{Result, WrapErr, bail, eyre};
use ros_z::dynamic::{
    ByteRenderPolicy, DynamicJsonRenderPolicy, DynamicNamedValue, DynamicPayload, DynamicStruct,
    DynamicSubscriber, DynamicValue, EnumPayloadValue, EnumValue, NonFiniteFloatRenderPolicy,
    dynamic_payload_to_json,
};

use crate::{
    app::AppContext,
    model::echo::{EchoHeader, EchoMessageView},
    render::{OutputMode, json, text},
};

const TYPE_DISCOVERY_TIMEOUT: Duration = Duration::from_secs(5);

const JSON_RENDER_POLICY: DynamicJsonRenderPolicy = DynamicJsonRenderPolicy {
    bytes: ByteRenderPolicy::FullArray,
    non_finite_float: NonFiniteFloatRenderPolicy::Null,
};

pub async fn run(
    app: &AppContext,
    output_mode: OutputMode,
    topic: &str,
    count: Option<usize>,
    timeout: Option<f64>,
) -> Result<()> {
    let subscriber = app
        .create_dynamic_subscriber(topic, TYPE_DISCOVERY_TIMEOUT)
        .await?;
    let _schema = subscriber
        .schema()
        .ok_or_else(|| eyre!("dynamic subscriber missing schema for {topic}"))?;
    let header = EchoHeader::new(
        topic.to_string(),
        subscriber.entity().type_info.name.clone(),
        subscriber.entity().type_info.hash.to_hash_string(),
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
                    subscriber.entity().type_info.name.clone(),
                    header.schema_hash.clone(),
                    dynamic_payload_to_json(&message, JSON_RENDER_POLICY),
                );
                json::print_line(&view)?;
            }
            OutputMode::Text => {
                text::print_echo_message(&format_payload_pretty(&message), count, seen);
            }
        }
    }
}

async fn receive_message(
    subscriber: &DynamicSubscriber,
    deadline: Option<tokio::time::Instant>,
    topic: &str,
) -> Result<DynamicPayload> {
    let receive = subscriber.recv();

    match deadline {
        Some(deadline) => match tokio::time::timeout_at(deadline, receive).await {
            Ok(result) => result.wrap_err_with(|| format!("subscriber receive failed for {topic}")),
            Err(_) => bail!("timed out waiting for messages on {topic}"),
        },
        None => receive
            .await
            .wrap_err_with(|| format!("subscriber receive failed for {topic}")),
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
