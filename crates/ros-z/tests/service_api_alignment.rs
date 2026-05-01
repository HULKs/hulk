use std::{sync::Arc, time::Duration};

use ros_z::{
    Message, ServiceTypeInfo,
    context::ContextBuilder,
    dynamic::{FieldType, MessageSchema},
    entity::SchemaHash,
    msg::Service,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
struct AddTwoIntsRequest {
    a: i64,
    b: i64,
}

impl Message for AddTwoIntsRequest {
    type Codec = ros_z::SerdeCdrCodec<Self>;

    fn type_name() -> &'static str {
        "test_msgs::AddTwoIntsRequest"
    }

    fn schema_hash() -> SchemaHash {
        SchemaHash::zero()
    }

    fn schema() -> std::sync::Arc<MessageSchema> {
        MessageSchema::builder("test_msgs::AddTwoIntsRequest")
            .field("a", FieldType::Int64)
            .field("b", FieldType::Int64)
            .build()
            .expect("schema should build")
    }
}

impl ros_z::msg::WireMessage for AddTwoIntsRequest {
    type Codec = ros_z::msg::SerdeCdrCodec<AddTwoIntsRequest>;
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
struct AddTwoIntsResponse {
    sum: i64,
}

impl Message for AddTwoIntsResponse {
    type Codec = ros_z::SerdeCdrCodec<Self>;

    fn type_name() -> &'static str {
        "test_msgs::AddTwoIntsResponse"
    }

    fn schema_hash() -> SchemaHash {
        SchemaHash::zero()
    }

    fn schema() -> std::sync::Arc<MessageSchema> {
        MessageSchema::builder("test_msgs::AddTwoIntsResponse")
            .field("sum", FieldType::Int64)
            .build()
            .expect("schema should build")
    }
}

impl ros_z::msg::WireMessage for AddTwoIntsResponse {
    type Codec = ros_z::msg::SerdeCdrCodec<AddTwoIntsResponse>;
}

struct AddTwoInts;

impl ServiceTypeInfo for AddTwoInts {
    fn service_type_info() -> ros_z::entity::TypeInfo {
        let descriptor = ros_z_schema::ServiceDef::new(
            "test_msgs::AddTwoInts",
            "test_msgs::AddTwoIntsRequest",
            "test_msgs::AddTwoIntsResponse",
        )
        .expect("service descriptor");
        ros_z::entity::TypeInfo::new(
            "test_msgs::AddTwoInts",
            Some(SchemaHash(ros_z_schema::compute_hash(&descriptor).0)),
        )
    }
}

impl Service for AddTwoInts {
    type Request = AddTwoIntsRequest;
    type Response = AddTwoIntsResponse;
}

#[test]
fn generated_service_and_manual_descriptor_share_the_same_hash() {
    let manual = ros_z_schema::ServiceDef::new(
        "test_msgs::AddTwoInts",
        "test_msgs::AddTwoIntsRequest",
        "test_msgs::AddTwoIntsResponse",
    )
    .expect("service descriptor");

    assert_eq!(
        AddTwoInts::service_type_info()
            .hash
            .expect("generated hash"),
        ros_z::entity::SchemaHash(ros_z_schema::compute_hash(&manual).0)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn blocking_and_async_service_calls_share_the_same_api_names() {
    let context = ContextBuilder::default()
        .disable_multicast_scouting()
        .with_json("connect/endpoints", json!([]))
        .build()
        .await
        .expect("Failed to create context");

    let server_ctx = context.clone();
    let server_handle = tokio::spawn(async move {
        let node = server_ctx
            .create_node("alignment_server")
            .build()
            .await
            .expect("Failed to create server node");

        let mut server = node
            .create_service_server::<AddTwoInts>("service_api_alignment")
            .build()
            .await
            .expect("Failed to create service server");

        for (a, b) in [(10, 32), (20, 22)] {
            let request = server
                .take_request_async()
                .await
                .expect("Failed to take request");
            assert_eq!(request.message().a, a);
            assert_eq!(request.message().b, b);

            request
                .reply_async(&AddTwoIntsResponse { sum: a + b })
                .await
                .expect("Failed to send response");
        }
    });

    let client_node = context
        .create_node("alignment_client")
        .build()
        .await
        .expect("Failed to create client node");

    let client = Arc::new(
        client_node
            .create_service_client::<AddTwoInts>("service_api_alignment")
            .build()
            .await
            .expect("Failed to create service client"),
    );

    tokio::time::sleep(Duration::from_millis(100)).await;

    let async_response = client
        .call_with_timeout_async(&AddTwoIntsRequest { a: 10, b: 32 }, Duration::from_secs(2))
        .await
        .expect("Failed to receive async response");
    assert_eq!(async_response.sum, 42);

    let blocking_client = Arc::clone(&client);
    let blocking_response = tokio::task::spawn_blocking(move || {
        blocking_client
            .call_with_timeout(&AddTwoIntsRequest { a: 20, b: 22 }, Duration::from_secs(2))
    })
    .await
    .expect("Blocking call task panicked")
    .expect("Failed to receive blocking response");
    assert_eq!(blocking_response.sum, 42);

    server_handle.await.expect("Server task panicked");
}
