use std::{thread, time::Duration};

use ros_z::__private::ros_z_schema::TypeName;
use ros_z::{
    Message, ServiceTypeInfo,
    context::ContextBuilder,
    dynamic::{RuntimeFieldSchema, Schema, TypeShape},
    entity::SchemaHash,
    msg::Service,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

fn struct_schema(name: &str, fields: Vec<RuntimeFieldSchema>) -> Schema {
    std::sync::Arc::new(TypeShape::Struct {
        name: TypeName::new(name.to_string()).expect("valid test type name"),
        fields,
    })
}

// Simple test service request
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

    fn schema() -> Schema {
        struct_schema(
            "test_msgs::AddTwoIntsRequest",
            vec![
                RuntimeFieldSchema::new("a", i64::schema()),
                RuntimeFieldSchema::new("b", i64::schema()),
            ],
        )
    }
}

impl ros_z::msg::WireMessage for AddTwoIntsRequest {
    type Codec = ros_z::msg::SerdeCdrCodec<AddTwoIntsRequest>;
}

// Simple test service response
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

    fn schema() -> Schema {
        struct_schema(
            "test_msgs::AddTwoIntsResponse",
            vec![RuntimeFieldSchema::new("sum", i64::schema())],
        )
    }
}

impl ros_z::msg::WireMessage for AddTwoIntsResponse {
    type Codec = ros_z::msg::SerdeCdrCodec<AddTwoIntsResponse>;
}

// Service type definition
struct AddTwoInts;

impl ServiceTypeInfo for AddTwoInts {
    fn service_type_info() -> ros_z::entity::TypeInfo {
        ros_z::entity::TypeInfo::new("test_msgs::AddTwoInts", None)
    }
}

impl Service for AddTwoInts {
    type Request = AddTwoIntsRequest;
    type Response = AddTwoIntsResponse;
}

#[test]
fn manual_service_uses_native_service_trait() {
    fn assert_native_service<T: ros_z::Service>() {}

    assert_native_service::<AddTwoInts>();
}

#[tokio::test(flavor = "multi_thread")]
async fn test_basic_service_request_response() {
    let context = ContextBuilder::default()
        .disable_multicast_scouting()
        .with_json("connect/endpoints", json!([]))
        .build()
        .await
        .expect("Failed to create context");
    let handle = tokio::runtime::Handle::current();

    let server_handle = thread::spawn({
        let context = context.clone();
        let handle = handle.clone();
        move || {
            let node = handle
                .block_on(context.create_node("test_server").build())
                .expect("Failed to create node");

            let mut server = handle
                .block_on(
                    node.create_service_server::<AddTwoInts>("add_two_ints")
                        .build(),
                )
                .expect("Failed to create server");

            // Wait for request
            let request = server.take_request().expect("Failed to take request");
            assert_eq!(request.message().a, 10);
            assert_eq!(request.message().b, 32);

            let response = AddTwoIntsResponse {
                sum: request.message().a + request.message().b,
            };
            request.reply(&response).expect("Failed to send response");
        }
    });

    let client_handle = thread::spawn({
        let context = context.clone();
        let handle = handle.clone();
        move || {
            let node = handle
                .block_on(context.create_node("test_client").build())
                .expect("Failed to create node");

            let client = handle
                .block_on(
                    node.create_service_client::<AddTwoInts>("add_two_ints")
                        .build(),
                )
                .expect("Failed to create client");

            // Give server time to start
            thread::sleep(Duration::from_millis(100));

            let request = AddTwoIntsRequest { a: 10, b: 32 };

            let response = client
                .call_with_timeout(&request, Duration::from_secs(2))
                .expect("Failed to receive response");

            assert_eq!(response.sum, 42);
        }
    });

    server_handle.join().expect("Server thread panicked");
    client_handle.join().expect("Client thread panicked");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_async_service_request_response() {
    let context = ContextBuilder::default()
        .disable_multicast_scouting()
        .with_json("connect/endpoints", json!([]))
        .build()
        .await
        .expect("Failed to create context");

    let server_ctx = context.clone();
    let server_handle = tokio::spawn(async move {
        let node = server_ctx
            .create_node("async_server")
            .build()
            .await
            .expect("Failed to create node");

        let mut server = node
            .create_service_server::<AddTwoInts>("async_add")
            .build()
            .await
            .expect("Failed to create server");

        // Wait for request asynchronously
        let request = server
            .take_request_async()
            .await
            .expect("Failed to take request");
        let response = AddTwoIntsResponse {
            sum: request.message().a + request.message().b,
        };

        request
            .reply_async(&response)
            .await
            .expect("Failed to send response");
    });

    let client_ctx = context.clone();
    let client_handle = tokio::spawn(async move {
        let node = client_ctx
            .create_node("async_client")
            .build()
            .await
            .expect("Failed to create node");

        let client = node
            .create_service_client::<AddTwoInts>("async_add")
            .build()
            .await
            .expect("Failed to create client");

        // Give server time to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        let request = AddTwoIntsRequest { a: 100, b: 23 };
        let response = client
            .call_async(&request)
            .await
            .expect("Failed to receive response");

        assert_eq!(response.sum, 123);
    });

    let (server_result, client_result) = tokio::join!(server_handle, client_handle);
    server_result.expect("Server task panicked");
    client_result.expect("Client task panicked");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_multiple_service_requests() {
    let context = ContextBuilder::default()
        .disable_multicast_scouting()
        .with_json("connect/endpoints", json!([]))
        .build()
        .await
        .expect("Failed to create context");
    let handle = tokio::runtime::Handle::current();

    let server_handle = thread::spawn({
        let context = context.clone();
        let handle = handle.clone();
        move || {
            let node = handle
                .block_on(context.create_node("multi_server").build())
                .expect("Failed to create node");

            let mut server = handle
                .block_on(
                    node.create_service_server::<AddTwoInts>("multi_add")
                        .build(),
                )
                .expect("Failed to create server");

            // Handle 3 requests
            for expected_a in [1, 2, 3] {
                let request = server.take_request().expect("Failed to take request");
                assert_eq!(request.message().a, expected_a);
                assert_eq!(request.message().b, 10);

                let response = AddTwoIntsResponse {
                    sum: request.message().a + request.message().b,
                };
                request.reply(&response).expect("Failed to send response");
            }
        }
    });

    let client_handle = thread::spawn({
        let context = context.clone();
        let handle = handle.clone();
        move || {
            let node = handle
                .block_on(context.create_node("multi_client").build())
                .expect("Failed to create node");

            let client = handle
                .block_on(
                    node.create_service_client::<AddTwoInts>("multi_add")
                        .build(),
                )
                .expect("Failed to create client");

            // Give server time to start
            thread::sleep(Duration::from_millis(100));

            // Send 3 requests sequentially
            for a in [1, 2, 3] {
                let request = AddTwoIntsRequest { a, b: 10 };

                let response = client
                    .call_with_timeout(&request, Duration::from_secs(2))
                    .expect("Failed to receive response");

                assert_eq!(response.sum, a + 10);
            }
        }
    });

    server_handle.join().expect("Server thread panicked");
    client_handle.join().expect("Client thread panicked");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_blocking_call_waits_for_service_response() {
    let context = ContextBuilder::default()
        .disable_multicast_scouting()
        .with_json("connect/endpoints", json!([]))
        .build()
        .await
        .expect("Failed to create context");
    let handle = tokio::runtime::Handle::current();

    let server_handle = thread::spawn({
        let context = context.clone();
        let handle = handle.clone();
        move || {
            let node = handle
                .block_on(context.create_node("blocking_call_server").build())
                .expect("Failed to create node");

            let mut server = handle
                .block_on(
                    node.create_service_server::<AddTwoInts>("blocking_call")
                        .build(),
                )
                .expect("Failed to create server");

            let request = server.take_request().expect("Failed to take request");
            assert_eq!(request.message().a, 40);
            assert_eq!(request.message().b, 2);

            request
                .reply(&AddTwoIntsResponse { sum: 42 })
                .expect("Failed to send response");
        }
    });

    let client_handle = thread::spawn({
        let context = context.clone();
        let handle = handle.clone();
        move || {
            let node = handle
                .block_on(context.create_node("blocking_call_client").build())
                .expect("Failed to create node");

            let client = handle
                .block_on(
                    node.create_service_client::<AddTwoInts>("blocking_call")
                        .build(),
                )
                .expect("Failed to create client");

            thread::sleep(Duration::from_millis(100));

            let response = client
                .call(&AddTwoIntsRequest { a: 40, b: 2 })
                .expect("Failed to receive blocking response");

            assert_eq!(response, AddTwoIntsResponse { sum: 42 });
        }
    });

    server_handle.join().expect("Server thread panicked");
    client_handle.join().expect("Client thread panicked");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_blocking_call_with_timeout_can_exceed_old_builder_timeout() {
    let context = ContextBuilder::default()
        .disable_multicast_scouting()
        .with_json("connect/endpoints", json!([]))
        .build()
        .await
        .expect("Failed to create context");
    let handle = tokio::runtime::Handle::current();

    let server_handle = thread::spawn({
        let context = context.clone();
        let handle = handle.clone();
        move || {
            let node = handle
                .block_on(context.create_node("builder_timeout_queue_server").build())
                .expect("Failed to create node");

            let mut server = handle
                .block_on(
                    node.create_service_server::<AddTwoInts>("builder_timeout_queue")
                        .build(),
                )
                .expect("Failed to create server");

            let request = server.take_request().expect("Failed to take request");
            assert_eq!(request.message(), &AddTwoIntsRequest { a: 1, b: 2 });

            thread::sleep(Duration::from_secs(11));

            request
                .reply(&AddTwoIntsResponse { sum: 3 })
                .expect("Failed to send delayed response");
        }
    });

    let node = context
        .create_node("builder_timeout_queue_client")
        .build()
        .await
        .expect("Failed to create node");

    let client = node
        .create_service_client::<AddTwoInts>("builder_timeout_queue")
        .build()
        .await
        .expect("Failed to create client");

    thread::sleep(Duration::from_millis(100));

    let response = tokio::task::spawn_blocking(move || {
        client.call_with_timeout(&AddTwoIntsRequest { a: 1, b: 2 }, Duration::from_secs(12))
    })
    .await
    .expect("blocking client task panicked")
    .expect("Expected delayed blocking response before user timeout");

    assert_eq!(response, AddTwoIntsResponse { sum: 3 });

    server_handle.join().expect("Server thread panicked");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_async_call_with_timeout_can_exceed_old_builder_timeout() {
    let context = ContextBuilder::default()
        .disable_multicast_scouting()
        .with_json("connect/endpoints", json!([]))
        .build()
        .await
        .expect("Failed to create context");

    let server_ctx = context.clone();
    let server_handle = tokio::spawn(async move {
        let node = server_ctx
            .create_node("builder_timeout_async_server")
            .build()
            .await
            .expect("Failed to create node");

        let mut server = node
            .create_service_server::<AddTwoInts>("builder_timeout_async")
            .build()
            .await
            .expect("Failed to create server");

        let request = server
            .take_request_async()
            .await
            .expect("Failed to take request");
        assert_eq!(request.message(), &AddTwoIntsRequest { a: 1, b: 2 });

        tokio::time::sleep(Duration::from_secs(11)).await;

        request
            .reply_async(&AddTwoIntsResponse { sum: 3 })
            .await
            .expect("Failed to send delayed response");
    });

    let client_ctx = context.clone();
    let client_handle = tokio::spawn(async move {
        let node = client_ctx
            .create_node("builder_timeout_async_client")
            .build()
            .await
            .expect("Failed to create node");

        let client = node
            .create_service_client::<AddTwoInts>("builder_timeout_async")
            .build()
            .await
            .expect("Failed to create client");

        tokio::time::sleep(Duration::from_millis(100)).await;

        let response = client
            .call_with_timeout_async(&AddTwoIntsRequest { a: 1, b: 2 }, Duration::from_secs(12))
            .await
            .expect("Expected delayed async response before user timeout");

        assert_eq!(response, AddTwoIntsResponse { sum: 3 });
    });

    let (server_result, client_result) = tokio::join!(server_handle, client_handle);
    server_result.expect("Server task panicked");
    client_result.expect("Client task panicked");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_blocking_call_with_timeout_reports_early_completion_when_no_server_replies() {
    let context = ContextBuilder::default()
        .disable_multicast_scouting()
        .with_json("connect/endpoints", json!([]))
        .build()
        .await
        .expect("Failed to create context");

    let node = context
        .create_node("blocking_timeout_client")
        .build()
        .await
        .expect("Failed to create node");

    let client = node
        .create_service_client::<AddTwoInts>("blocking_timeout")
        .build()
        .await
        .expect("Failed to create client");

    let error = tokio::task::spawn_blocking(move || {
        client.call_with_timeout(
            &AddTwoIntsRequest { a: 1, b: 2 },
            Duration::from_millis(200),
        )
    })
    .await
    .expect("blocking client task panicked")
    .expect_err("Expected blocking failure without a server response");

    assert_eq!(
        error.to_string(),
        "Service call ended before any response was received"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_blocking_call_with_timeout_returns_response_before_deadline() {
    let context = ContextBuilder::default()
        .disable_multicast_scouting()
        .with_json("connect/endpoints", json!([]))
        .build()
        .await
        .expect("Failed to create context");
    let handle = tokio::runtime::Handle::current();

    let server_handle = thread::spawn({
        let context = context.clone();
        let handle = handle.clone();
        move || {
            let node = handle
                .block_on(
                    context
                        .create_node("blocking_timeout_success_server")
                        .build(),
                )
                .expect("Failed to create node");

            let mut server = handle
                .block_on(
                    node.create_service_server::<AddTwoInts>("blocking_timeout_success")
                        .build(),
                )
                .expect("Failed to create server");

            let request = server.take_request().expect("Failed to take request");
            thread::sleep(Duration::from_millis(100));
            request
                .reply(&AddTwoIntsResponse { sum: 42 })
                .expect("Failed to send response");
        }
    });

    let node = context
        .create_node("blocking_timeout_success_client")
        .build()
        .await
        .expect("Failed to create node");

    let client = node
        .create_service_client::<AddTwoInts>("blocking_timeout_success")
        .build()
        .await
        .expect("Failed to create client");

    thread::sleep(Duration::from_millis(100));

    let response = tokio::task::spawn_blocking(move || {
        client.call_with_timeout(&AddTwoIntsRequest { a: 40, b: 2 }, Duration::from_secs(1))
    })
    .await
    .expect("blocking client task panicked")
    .expect("Expected blocking response before timeout");

    assert_eq!(response, AddTwoIntsResponse { sum: 42 });

    server_handle.join().expect("Server thread panicked");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_blocking_call_with_timeout_reports_real_timeout_while_waiting_for_reply() {
    let context = ContextBuilder::default()
        .disable_multicast_scouting()
        .with_json("connect/endpoints", json!([]))
        .build()
        .await
        .expect("Failed to create context");
    let handle = tokio::runtime::Handle::current();

    let server_handle = thread::spawn({
        let context = context.clone();
        let handle = handle.clone();
        move || {
            let node = handle
                .block_on(
                    context
                        .create_node("blocking_timeout_waiting_server")
                        .build(),
                )
                .expect("Failed to create node");

            let mut server = handle
                .block_on(
                    node.create_service_server::<AddTwoInts>("blocking_timeout_waiting")
                        .build(),
                )
                .expect("Failed to create server");

            let request = server.take_request().expect("Failed to take request");
            thread::sleep(Duration::from_millis(500));
            request
                .reply(&AddTwoIntsResponse { sum: 3 })
                .expect("Failed to send delayed response");
        }
    });

    let node = context
        .create_node("blocking_timeout_waiting_client")
        .build()
        .await
        .expect("Failed to create node");

    let client = node
        .create_service_client::<AddTwoInts>("blocking_timeout_waiting")
        .build()
        .await
        .expect("Failed to create client");

    thread::sleep(Duration::from_millis(100));

    let error = tokio::task::spawn_blocking(move || {
        client.call_with_timeout(
            &AddTwoIntsRequest { a: 1, b: 2 },
            Duration::from_millis(100),
        )
    })
    .await
    .expect("blocking client task panicked")
    .expect_err("Expected blocking timeout while reply was still pending");

    assert!(
        error.to_string().contains("timed out"),
        "unexpected timeout error: {error}"
    );

    server_handle.join().expect("Server thread panicked");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_blocking_call_with_timeout_reports_early_completion_without_reply() {
    let context = ContextBuilder::default()
        .disable_multicast_scouting()
        .with_json("connect/endpoints", json!([]))
        .build()
        .await
        .expect("Failed to create context");

    let server = context
        .create_node("blocking_early_completion_server")
        .build()
        .await
        .expect("Failed to create node")
        .create_service_server::<AddTwoInts>("blocking_early_completion")
        .build_with_callback(move |_query| {
            // Intentionally end the query without producing a successful reply sample.
        })
        .await
        .expect("Failed to create callback server");

    let _server = server;

    let node = context
        .create_node("blocking_early_completion_client")
        .build()
        .await
        .expect("Failed to create node");

    let client = node
        .create_service_client::<AddTwoInts>("blocking_early_completion")
        .build()
        .await
        .expect("Failed to create client");

    thread::sleep(Duration::from_millis(100));

    let error = tokio::task::spawn_blocking(move || {
        client.call_with_timeout(&AddTwoIntsRequest { a: 1, b: 2 }, Duration::from_secs(1))
    })
    .await
    .expect("blocking client task panicked")
    .expect_err("Expected early completion without any reply sample");

    assert_eq!(
        error.to_string(),
        "Service call ended before any response was received"
    );
}
