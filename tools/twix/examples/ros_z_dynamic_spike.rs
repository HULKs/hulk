use std::time::Duration;

use clap::Parser;
use color_eyre::eyre::{Result, WrapErr, bail, eyre};
use ros_z::{
    context::{Context, ContextBuilder},
    dynamic::{
        DynamicPayload, DynamicStruct, DynamicValue, EnumPayloadValue, EnumValue, SchemaBundle,
        TypeDef,
    },
};
use serde_json::{Map, Value};

#[derive(Debug, Parser)]
struct Args {
    #[arg(long, default_value = "tcp/127.0.0.1:7447")]
    endpoint: String,

    #[arg(long, default_value = "/robot_status")]
    topic: String,

    #[arg(long, default_value = "5")]
    discovery_timeout_secs: u64,

    #[arg(long, default_value = "5")]
    graph_wait_secs: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let args = Args::parse();
    let graph_wait = Duration::from_secs(args.graph_wait_secs);
    let discovery_timeout = Duration::from_secs(args.discovery_timeout_secs);

    let context = ContextBuilder::default()
        .with_mode("client")
        .with_connect_endpoints([args.endpoint.as_str()])
        .build()
        .await
        .map_err(|error| eyre!(error))
        .wrap_err("failed to build ros-z context")?;
    let node = context
        .create_node("twix_dynamic_spike")
        .with_namespace("tools")
        .build()
        .await
        .map_err(|error| eyre!(error))
        .wrap_err("failed to build ros-z node")?;

    wait_for_topic(&context, &args.topic, graph_wait).await?;

    let topics = context.graph().get_topic_names_and_types();
    let visible_topic = topics
        .iter()
        .find(|(topic, _)| topic == &args.topic)
        .ok_or_else(|| eyre!("topic {} was not visible after wait", args.topic))?;

    println!("graph topic: {}", visible_topic.0);
    println!("graph type: {}", visible_topic.1);

    let subscriber = node
        .dynamic_subscriber_auto(&args.topic, discovery_timeout)
        .await
        .map_err(|error| eyre!(error))
        .wrap_err_with(|| format!("failed to discover schema for {}", args.topic))?
        .build()
        .await
        .map_err(|error| eyre!(error))
        .wrap_err_with(|| format!("failed to build dynamic subscriber for {}", args.topic))?;

    let schema = subscriber
        .schema()
        .ok_or_else(|| eyre!("dynamic subscriber did not retain its schema"))?;

    println!("schema type: {}", schema_type_name(schema));

    let received = subscriber
        .recv_with_metadata()
        .await
        .map_err(|error| eyre!(error))
        .wrap_err("failed to receive dynamic sample with metadata")?;

    println!("transport time: {:?}", received.transport_time);
    println!("source time: {:?}", received.source_time);
    println!("sequence number: {:?}", received.sequence_number);
    println!("source global id: {:?}", received.source_global_id);
    println!(
        "json: {}",
        serde_json::to_string_pretty(&dynamic_payload_to_json(&received.message))?
    );

    Ok(())
}

async fn wait_for_topic(context: &Context, topic: &str, timeout: Duration) -> Result<()> {
    let deadline = tokio::time::Instant::now() + timeout;

    loop {
        if context
            .graph()
            .get_topic_names_and_types()
            .iter()
            .any(|(visible_topic, _)| visible_topic == topic)
        {
            return Ok(());
        }

        if tokio::time::Instant::now() >= deadline {
            let visible_topics = context
                .graph()
                .get_topic_names_and_types()
                .into_iter()
                .map(|(topic_name, type_name)| format!("{topic_name} ({type_name})"))
                .collect::<Vec<_>>();
            bail!(
                "timed out waiting for topic {topic}; visible topics: {}",
                visible_topics.join(", ")
            );
        }

        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

fn dynamic_payload_to_json(message: &DynamicPayload) -> Value {
    dynamic_value_to_json(&message.value)
}

fn schema_type_name(schema: &SchemaBundle) -> &str {
    match &schema.root {
        TypeDef::Named(name) => name.as_str(),
        TypeDef::Primitive(_) => "<primitive>",
        TypeDef::String => "<string>",
        TypeDef::Optional(_) => "<optional>",
        TypeDef::Sequence { .. } => "<sequence>",
        TypeDef::Map { .. } => "<map>",
    }
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
