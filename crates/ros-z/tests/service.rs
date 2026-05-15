use std::{thread, time::Duration};

use ros_z::{
    Message, ServiceTypeInfo, context::ContextBuilder, entity::TypeInfo, message::Service,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use zenoh::Wait;

// Simple test service request
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, ros_z::Message)]
#[message(name = "test_msgs::AddTwoIntsRequest")]
struct AddTwoIntsRequest {
    a: i64,
    b: i64,
}

// Simple test service response
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, ros_z::Message)]
#[message(name = "test_msgs::AddTwoIntsResponse")]
struct AddTwoIntsResponse {
    sum: i64,
}

// Service type definition
struct AddTwoInts;

impl ServiceTypeInfo for AddTwoInts {
    fn service_type_info() -> TypeInfo {
        let descriptor = ros_z_schema::ServiceDef::new(
            "test_msgs::AddTwoInts",
            AddTwoIntsRequest::type_name(),
            AddTwoIntsResponse::type_name(),
        )
        .expect("test service descriptor should be static and valid");
        let hash = ros_z_schema::compute_hash(&descriptor)
            .expect("test service hash should be static and valid");
        TypeInfo::new(descriptor.type_name.as_str(), hash)
    }
}

impl Service for AddTwoInts {
    type Request = AddTwoIntsRequest;
    type Response = AddTwoIntsResponse;
}

struct InvalidServiceTypeInfo;

impl ServiceTypeInfo for InvalidServiceTypeInfo {
    fn service_type_info() -> TypeInfo {
        let descriptor = ros_z_schema::ServiceDef::new(
            "",
            AddTwoIntsRequest::type_name(),
            AddTwoIntsResponse::type_name(),
        )
        .expect("test service descriptor should be static and valid");
        let hash = ros_z_schema::compute_hash(&descriptor)
            .expect("test service hash should be static and valid");
        TypeInfo::new(descriptor.type_name.as_str(), hash)
    }
}

impl Service for InvalidServiceTypeInfo {
    type Request = AddTwoIntsRequest;
    type Response = AddTwoIntsResponse;
}

fn assert_service_timeout(error: &ros_z::Error, expected_service: &str) {
    match error {
        ros_z::Error::ServiceCall(ros_z::error::ServiceCallError::Timeout { service, .. }) => {
            assert_eq!(service, expected_service);
        }
        other => panic!("expected service timeout for {expected_service}, got {other:?}"),
    }
}

fn assert_service_no_response(error: &ros_z::Error, expected_service: &str) {
    match error {
        ros_z::Error::ServiceCall(ros_z::error::ServiceCallError::NoResponse { service }) => {
            assert_eq!(service, expected_service);
        }
        other => panic!("expected service no-response for {expected_service}, got {other:?}"),
    }
}

fn assert_service_reply_error<'a>(
    error: &'a ros_z::Error,
    expected_service: &str,
) -> &'a zenoh::query::ReplyError {
    match error {
        ros_z::Error::ServiceCall(ros_z::error::ServiceCallError::Reply { service, source }) => {
            assert_eq!(service, expected_service);
            source
                .downcast_ref::<zenoh::query::ReplyError>()
                .expect("service reply source should be a Zenoh reply error")
        }
        other => panic!("expected service reply error for {expected_service}, got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[should_panic(expected = "test service descriptor should be static and valid")]
async fn typed_service_server_panics_for_invalid_static_service_type_info() {
    let context = ContextBuilder::default()
        .disable_multicast_scouting()
        .with_json("connect/endpoints", json!([]))
        .build()
        .await
        .expect("Failed to create context");
    let node = context
        .create_node("invalid_service_schema")
        .build()
        .await
        .expect("Failed to create node");

    let _ = node.create_service_server::<InvalidServiceTypeInfo>("invalid_service");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[should_panic(expected = "test service descriptor should be static and valid")]
async fn typed_service_client_panics_for_invalid_static_service_type_info() {
    let context = ContextBuilder::default()
        .disable_multicast_scouting()
        .with_json("connect/endpoints", json!([]))
        .build()
        .await
        .expect("Failed to create context");
    let node = context
        .create_node("invalid_service_schema")
        .build()
        .await
        .expect("Failed to create node");

    let _ = node.create_service_client::<InvalidServiceTypeInfo>("invalid_service");
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
                        .expect("endpoint factory should succeed")
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
                        .expect("endpoint factory should succeed")
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
            .expect("endpoint factory should succeed")
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
            .expect("endpoint factory should succeed")
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
                        .expect("endpoint factory should succeed")
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
                        .expect("endpoint factory should succeed")
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
                        .expect("endpoint factory should succeed")
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
                        .expect("endpoint factory should succeed")
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
                        .expect("endpoint factory should succeed")
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
        .expect("endpoint factory should succeed")
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
            .expect("endpoint factory should succeed")
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
            .expect("endpoint factory should succeed")
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
async fn test_blocking_call_with_timeout_reports_timeout_when_no_service_matches() {
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
        .expect("endpoint factory should succeed")
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

    assert_service_timeout(&error, "/blocking_timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn async_call_with_timeout_reports_timeout_when_no_service_matches() {
    let context = ContextBuilder::default()
        .disable_multicast_scouting()
        .with_json("connect/endpoints", json!([]))
        .build()
        .await
        .expect("Failed to create context");

    let client = context
        .create_node("async_timeout_client")
        .build()
        .await
        .expect("Failed to create node")
        .create_service_client::<AddTwoInts>("async_timeout")
        .expect("endpoint factory should succeed")
        .build()
        .await
        .expect("Failed to create client");

    let error = client
        .call_with_timeout_async(
            &AddTwoIntsRequest { a: 1, b: 2 },
            Duration::from_millis(200),
        )
        .await
        .expect_err("Expected async timeout without a matching service");

    assert_service_timeout(&error, "/async_timeout");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn async_call_preserves_service_reply_error_source() {
    let context = ContextBuilder::default()
        .disable_multicast_scouting()
        .with_json("connect/endpoints", json!([]))
        .build()
        .await
        .expect("Failed to create context");

    let _server = context
        .create_node("async_reply_error_server")
        .build()
        .await
        .expect("Failed to create node")
        .create_service_server::<AddTwoInts>("async_reply_error")
        .expect("endpoint factory should succeed")
        .build_with_callback(move |query| {
            query
                .reply_err("intentional service failure")
                .wait()
                .expect("Failed to send service error reply");
        })
        .await
        .expect("Failed to create callback server");

    let client = context
        .create_node("async_reply_error_client")
        .build()
        .await
        .expect("Failed to create node")
        .create_service_client::<AddTwoInts>("async_reply_error")
        .expect("endpoint factory should succeed")
        .build()
        .await
        .expect("Failed to create client");

    tokio::time::sleep(Duration::from_millis(100)).await;

    let error = client
        .call_async(&AddTwoIntsRequest { a: 1, b: 2 })
        .await
        .expect_err("Expected service error reply");

    let source = assert_service_reply_error(&error, "/async_reply_error");
    assert!(source.to_string().contains("query returned an error"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn callback_service_server_queue_access_returns_state_error() {
    let context = ContextBuilder::default()
        .disable_multicast_scouting()
        .with_json("connect/endpoints", json!([]))
        .build()
        .await
        .expect("Failed to create context");
    let node = context
        .create_node("callback_queue_state_error")
        .build()
        .await
        .expect("Failed to create node");

    let mut server = node
        .create_service_server::<AddTwoInts>("callback_queue_state_error")
        .expect("service server factory should succeed")
        .build_with_callback(|_request| {})
        .await
        .expect("callback server should build");

    let error = match server.try_take_request() {
        Ok(_) => panic!("callback server should not expose request queue"),
        Err(error) => error,
    };

    match error {
        ros_z::Error::ServiceServerState { operation, reason } => {
            assert_eq!(operation, "access service request queue");
            assert_eq!(reason, "server was built with callback, no queue available");
        }
        other => panic!("expected service server state error, got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn async_call_with_timeout_preserves_timeout_reply_error() {
    let context = ContextBuilder::default()
        .disable_multicast_scouting()
        .with_json("connect/endpoints", json!([]))
        .build()
        .await
        .expect("Failed to create context");

    let _server = context
        .create_node("async_timeout_reply_error_server")
        .build()
        .await
        .expect("Failed to create node")
        .create_service_server::<AddTwoInts>("async_timeout_reply_error")
        .expect("endpoint factory should succeed")
        .build_with_callback(move |query| {
            query
                .reply_err("Timeout")
                .wait()
                .expect("Failed to send service error reply");
        })
        .await
        .expect("Failed to create callback server");

    let client = context
        .create_node("async_timeout_reply_error_client")
        .build()
        .await
        .expect("Failed to create node")
        .create_service_client::<AddTwoInts>("async_timeout_reply_error")
        .expect("endpoint factory should succeed")
        .build()
        .await
        .expect("Failed to create client");

    tokio::time::sleep(Duration::from_millis(100)).await;

    let error = client
        .call_with_timeout_async(&AddTwoIntsRequest { a: 1, b: 2 }, Duration::from_secs(1))
        .await
        .expect_err("Expected service error reply");

    let source = assert_service_reply_error(&error, "/async_timeout_reply_error");
    assert!(source.to_string().contains("query returned an error"));
    assert!(!matches!(
        error,
        ros_z::Error::ServiceCall(ros_z::error::ServiceCallError::Timeout { .. })
    ));
    assert!(!error.to_string().contains("timed out"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn blocking_call_preserves_service_reply_error_source() {
    let context = ContextBuilder::default()
        .disable_multicast_scouting()
        .with_json("connect/endpoints", json!([]))
        .build()
        .await
        .expect("Failed to create context");

    let _server = context
        .create_node("blocking_reply_error_server")
        .build()
        .await
        .expect("Failed to create node")
        .create_service_server::<AddTwoInts>("blocking_reply_error")
        .expect("endpoint factory should succeed")
        .build_with_callback(move |query| {
            query
                .reply_err("intentional service failure")
                .wait()
                .expect("Failed to send service error reply");
        })
        .await
        .expect("Failed to create callback server");

    let client = context
        .create_node("blocking_reply_error_client")
        .build()
        .await
        .expect("Failed to create node")
        .create_service_client::<AddTwoInts>("blocking_reply_error")
        .expect("endpoint factory should succeed")
        .build()
        .await
        .expect("Failed to create client");

    tokio::time::sleep(Duration::from_millis(100)).await;

    let error = tokio::task::spawn_blocking(move || {
        client.call_with_timeout(&AddTwoIntsRequest { a: 1, b: 2 }, Duration::from_secs(1))
    })
    .await
    .expect("blocking client task panicked")
    .expect_err("Expected service error reply");

    let source = assert_service_reply_error(&error, "/blocking_reply_error");
    assert!(source.to_string().contains("query returned an error"));
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
                        .expect("endpoint factory should succeed")
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
        .expect("endpoint factory should succeed")
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
                        .expect("endpoint factory should succeed")
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
        .expect("endpoint factory should succeed")
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

    assert_service_timeout(&error, "/blocking_timeout_waiting");

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
        .expect("endpoint factory should succeed")
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
        .expect("endpoint factory should succeed")
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

    assert_service_no_response(&error, "/blocking_early_completion");
    assert!(error.to_string().contains("ended before any response"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn async_call_with_timeout_reports_early_completion_without_reply() {
    let context = ContextBuilder::default()
        .disable_multicast_scouting()
        .with_json("connect/endpoints", json!([]))
        .build()
        .await
        .expect("Failed to create context");

    let _server = context
        .create_node("async_early_completion_server")
        .build()
        .await
        .expect("Failed to create node")
        .create_service_server::<AddTwoInts>("async_early_completion")
        .expect("endpoint factory should succeed")
        .build_with_callback(move |_query| {
            // Intentionally end the query without producing a successful reply sample.
        })
        .await
        .expect("Failed to create callback server");

    let client = context
        .create_node("async_early_completion_client")
        .build()
        .await
        .expect("Failed to create node")
        .create_service_client::<AddTwoInts>("async_early_completion")
        .expect("endpoint factory should succeed")
        .build()
        .await
        .expect("Failed to create client");

    tokio::time::sleep(Duration::from_millis(100)).await;

    let error = client
        .call_with_timeout_async(&AddTwoIntsRequest { a: 1, b: 2 }, Duration::from_secs(1))
        .await
        .expect_err("Expected early completion without any reply sample");

    assert_service_no_response(&error, "/async_early_completion");
    assert!(error.to_string().contains("ended before any response"));
    assert!(!error.to_string().contains("timed out"));
}
