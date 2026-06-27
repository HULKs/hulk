use std::{sync::Arc, time::Duration};

use ros_z::prelude::*;
use ros_z_debug::{CachedSubscriptionFactory, CachedSubscriptionOptions, RetentionPolicy};

#[allow(dead_code)]
fn typed_subscription_builder_can_be_named<'a, T>(
    builder: ros_z_debug::CachedTypedSubscriptionBuilder<'a, T>,
) -> ros_z_debug::CachedTypedSubscriptionBuilder<'a, T> {
    builder
}

#[allow(dead_code)]
fn dynamic_subscription_builder_can_be_named<'a>(
    builder: ros_z_debug::CachedDynamicSubscriptionBuilder<'a>,
) -> ros_z_debug::CachedDynamicSubscriptionBuilder<'a> {
    builder
}

fn string_message_schema() -> ros_z::dynamic::Schema {
    use ros_z_schema::{
        FieldDef, SchemaBundle, StructDef, TypeDef, TypeDefinition, TypeDefinitions, TypeName,
    };

    let name = TypeName::new("test_msgs::StringMessage").expect("valid type name");
    Arc::new(SchemaBundle {
        root: TypeDef::Named(name.clone()),
        definitions: TypeDefinitions::from([(
            name,
            TypeDefinition::Struct(StructDef {
                fields: vec![FieldDef::new("data", TypeDef::String)],
            }),
        )]),
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn typed_subscription_receives_latest_sample() {
    let context = ContextBuilder::default()
        .disable_multicast_scouting()
        .with_json("connect/endpoints", serde_json::json!([]))
        .build()
        .await
        .expect("context should build");
    let publisher_node = context
        .create_node("typed_pub")
        .build()
        .await
        .expect("publisher node");
    let subscriber_node = Arc::new(
        context
            .create_node("typed_sub")
            .build()
            .await
            .expect("subscriber node"),
    );
    let publisher = publisher_node
        .publisher::<String>("debug_text")
        .build()
        .await
        .expect("publisher");
    let factory =
        CachedSubscriptionFactory::new(subscriber_node, CachedSubscriptionOptions::default());
    let handle = factory
        .subscribe_typed::<String>("debug_text")
        .expect("subscription builder")
        .retention(RetentionPolicy::LatestOnly)
        .build()
        .await
        .expect("subscription should build");

    publisher
        .publish(&"hello".to_string())
        .await
        .expect("publish should work");

    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(record) = handle.latest() {
            assert_eq!(record.value, "hello");
            assert_eq!(record.metadata.resolved_topic, "/debug_text");
            assert_eq!(record.publication_id.sequence_number(), 0);
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "timed out waiting for sample"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    publisher
        .publish(&"goodbye".to_string())
        .await
        .expect("second publish should work");

    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(record) = handle.latest()
            && record.value == "goodbye"
        {
            assert_eq!(record.metadata.resolved_topic, "/debug_text");
            assert_eq!(record.publication_id.sequence_number(), 1);
            return;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "timed out waiting for latest sample replacement"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn typed_subscription_resolves_relative_topic_against_target_namespace() {
    let context = ContextBuilder::default()
        .disable_multicast_scouting()
        .with_json("connect/endpoints", serde_json::json!([]))
        .build()
        .await
        .expect("context should build");
    let publisher_node = context
        .create_node("target_namespace_pub")
        .build()
        .await
        .expect("publisher node");
    let subscriber_node = Arc::new(
        context
            .create_node("target_namespace_sub")
            .build()
            .await
            .expect("subscriber node"),
    );
    let publisher = publisher_node
        .publisher::<String>("/alpha/debug_text")
        .build()
        .await
        .expect("publisher");
    let options =
        CachedSubscriptionOptions::with_target_namespace("/alpha").expect("valid target namespace");
    let factory = CachedSubscriptionFactory::new(subscriber_node, options);
    let handle = factory
        .subscribe_typed::<String>("debug_text")
        .expect("subscription builder")
        .retention(RetentionPolicy::LatestOnly)
        .build()
        .await
        .expect("subscription should build");

    publisher
        .publish(&"hello target".to_string())
        .await
        .expect("publish should work");

    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(record) = handle.latest() {
            assert_eq!(record.value, "hello target");
            assert_eq!(record.metadata.resolved_topic, "/alpha/debug_text");
            assert_eq!(record.publication_id.sequence_number(), 0);
            return;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "timed out waiting for target namespace sample"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dynamic_subscription_renders_json_view() {
    use ros_z::dynamic::{DynamicPayload, DynamicStruct};

    let context = ContextBuilder::default()
        .disable_multicast_scouting()
        .with_json("connect/endpoints", serde_json::json!([]))
        .build()
        .await
        .expect("context should build");
    let publisher_node = context
        .create_node("dynamic_pub")
        .build()
        .await
        .expect("publisher node");
    let subscriber_node = Arc::new(
        context
            .create_node("dynamic_sub")
            .build()
            .await
            .expect("subscriber node"),
    );
    let schema = string_message_schema();
    let type_info = ros_z::TypeInfo::new(
        "test_msgs::StringMessage",
        ros_z_schema::compute_hash(schema.as_ref()).expect("schema hash"),
    );
    let publisher = publisher_node
        .dynamic_publisher("debug_dynamic", type_info, schema.clone())
        .build()
        .await
        .expect("dynamic publisher");
    let factory =
        CachedSubscriptionFactory::new(subscriber_node, CachedSubscriptionOptions::default());
    let json = factory
        .subscribe_dynamic("debug_dynamic")
        .expect("subscription builder")
        .retention(RetentionPolicy::LatestOnly)
        .build_json(Default::default())
        .await
        .expect("dynamic json subscription should build");
    let mut message = DynamicStruct::default_for_schema(&schema).expect("default dynamic struct");
    message.set("data", "hello").expect("set field");
    let payload = DynamicPayload::from_struct(message).expect("dynamic payload");

    publisher
        .publish(&payload)
        .await
        .expect("publish should work");

    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(value) = json.latest_json() {
            assert_eq!(value, serde_json::json!({ "data": "hello" }));
            return;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "timed out waiting for dynamic sample"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[test]
fn namespace_projection_keeps_global_topics_qualified() {
    let projected = ros_z_debug::TopicProjection::project("alpha", ["/alpha/foo", "/diagnostics"])
        .expect("topics should project");

    assert!(projected.iter().any(|topic| topic.display_name == "foo"));
    assert!(
        projected
            .iter()
            .any(|topic| topic.display_name == "/diagnostics")
    );
}
