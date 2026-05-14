fn source_chain_len(error: &(dyn std::error::Error + 'static)) -> usize {
    let mut count = 0;
    let mut current = error.source();
    while let Some(source) = current {
        count += 1;
        current = source.source();
    }
    count
}

#[test]
fn ros_z_error_is_std_error_send_sync_static() {
    fn assert_error<T: std::error::Error + Send + Sync + 'static>() {}
    assert_error::<ros_z::Error>();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn ros_z_result_converts_into_color_eyre_report_with_source_chain() -> color_eyre::Result<()>
{
    color_eyre::install().ok();

    let key_expr: zenoh::key_expr::KeyExpr<'static> = "@wrong/1/2".try_into().unwrap();

    let result: ros_z::Result<()> = Err(ros_z_protocol::format::parse_liveliness(&key_expr)
        .unwrap_err()
        .into());
    let report = result.map_err(color_eyre::Report::from).unwrap_err();
    let rendered = format!("{report:?}");

    assert!(rendered.contains("failed to parse ros-z liveliness key"));
    assert!(report.source().is_some());

    Ok(())
}

#[test]
fn protocol_liveliness_parse_error_is_source_preserving() {
    let key_expr: zenoh::key_expr::KeyExpr<'static> = "@wrong/1/2".try_into().unwrap();

    let error = ros_z_protocol::format::parse_liveliness(&key_expr).unwrap_err();

    assert!(
        error
            .to_string()
            .contains("failed to parse ros-z liveliness key")
    );
    assert!(source_chain_len(&error) >= 1);
}

#[test]
fn service_reply_error_preserves_source_chain() {
    let source: zenoh::Error = Box::new(std::io::Error::new(
        std::io::ErrorKind::Other,
        "query returned an error",
    ));
    let error: ros_z::Error = ros_z::error::ServiceCallError::Reply {
        service: "/demo/service".to_string(),
        source,
    }
    .into();

    assert!(
        error
            .to_string()
            .contains("service call to '/demo/service' received an error reply")
    );
    assert!(source_chain_len(&error) >= 1);
}

#[test]
fn cdr_runtime_decode_error_preserves_source_chain() {
    let bytes = [0x00, 0x01, 0x00, 0x00, 0xff];
    let source = <String as ros_z::Message>::Codec::deserialize(&bytes)
        .expect_err("invalid payload should not decode");
    let error: ros_z::Error = ros_z::error::WireError::Decode {
        type_name: "String".to_string(),
        source: Box::new(source),
    }
    .into();

    assert!(
        error
            .to_string()
            .contains("failed to decode payload as String")
    );
    assert!(source_chain_len(&error) >= 1);
}

#[test]
fn parameter_parse_error_preserves_source_chain() {
    let source = json5::from_str::<serde_json::Value>("{ not json }").expect_err("invalid JSON5");
    let error: ros_z::Error = ros_z::parameter::ParameterError::ParseError {
        path: "params.json5".into(),
        source,
    }
    .into();

    assert!(
        error
            .to_string()
            .contains("failed to parse parameter file params.json5")
    );
    assert!(source_chain_len(&error) >= 1);
}

#[test]
fn parameter_operation_error_preserves_source_chain() {
    let source = std::io::Error::new(std::io::ErrorKind::Other, "runtime failed");
    let error: ros_z::Error = ros_z::parameter::ParameterError::Operation {
        operation: "calling remote parameter service".to_string(),
        source: Box::new(source),
    }
    .into();

    assert!(
        error
            .to_string()
            .contains("parameter operation failed while calling remote parameter service")
    );
    assert!(source_chain_len(&error) >= 1);
}

#[test]
fn cdr_decode_error_preserves_inner_cdr_source() {
    let bytes = [0x00, 0x01, 0x00, 0x00, 0xff];

    let error = <String as ros_z::Message>::Codec::deserialize(&bytes)
        .expect_err("invalid payload should not decode");

    assert!(error.to_string().contains("CDR deserialization error"));
    assert!(std::error::Error::source(&error).is_some());
}

#[test]
fn cdr_header_validation_error_has_no_inner_source() {
    let bytes = [0x00, 0x01, 0x00];

    let error = <String as ros_z::Message>::Codec::deserialize(&bytes)
        .expect_err("short header should not decode");

    assert!(error.to_string().contains("CDR data too short"));
    assert!(std::error::Error::source(&error).is_none());
}

#[test]
fn fallible_wire_encoder_preserves_encode_source() {
    let message = FallibleEncode { value: 42 };
    let source = FailingEncodeCodec::serialize(&message).unwrap_err();
    let error: ros_z::Error = ros_z::error::WireError::Encode {
        type_name: "test_msgs::FallibleEncode".to_string(),
        source: Box::new(source),
    }
    .into();

    assert!(
        error
            .to_string()
            .contains("failed to encode test_msgs::FallibleEncode")
    );
    let source = std::error::Error::source(&error).expect("encode error should preserve source");
    assert!(source.to_string().contains("intentional encode failure"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn publisher_build_invalid_topic_preserves_name_source() -> color_eyre::Result<()> {
    color_eyre::install().ok();

    let context = ros_z::context::ContextBuilder::default()
        .disable_multicast_scouting()
        .with_connect_endpoints(std::iter::empty::<&str>())
        .with_listen_endpoints(["tcp/127.0.0.1:0"])
        .build()
        .await?;
    let node = context.create_node("test_node").build().await?;

    let error = node
        .publisher::<String>("invalid-topic")
        .expect("publisher factory should succeed")
        .build()
        .await
        .expect_err("invalid topic should fail");

    assert!(error.to_string().contains("failed to qualify topic name"));
    assert!(source_chain_len(&error) >= 1);

    Ok(())
}

use ros_z::message::{WireDecoder, WireEncoder};
use serde::{Deserialize, Serialize};
use zenoh_buffers::ZBuf;

#[derive(Debug, thiserror::Error)]
#[error("intentional encode failure")]
struct IntentionalEncodeFailure;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ros_z::Message)]
#[message(name = "test_msgs::FallibleEncode")]
struct FallibleEncode {
    value: u32,
}

struct FailingEncodeCodec;

impl WireEncoder for FailingEncodeCodec {
    type Input<'a> = &'a FallibleEncode;
    type Error = IntentionalEncodeFailure;

    fn serialize_to_zbuf(_input: Self::Input<'_>) -> Result<ZBuf, Self::Error> {
        Err(IntentionalEncodeFailure)
    }

    fn serialize_to_zbuf_with_hint(
        input: Self::Input<'_>,
        _capacity_hint: usize,
    ) -> Result<ZBuf, Self::Error> {
        Self::serialize_to_zbuf(input)
    }

    fn serialize_to_shm(
        input: Self::Input<'_>,
        _estimated_size: usize,
        _provider: &zenoh::shm::ShmProvider<zenoh::shm::PosixShmProviderBackend>,
    ) -> ros_z::Result<(ZBuf, usize)> {
        let _ = input;
        Err(ros_z::Error::from(ros_z::error::WireError::Encode {
            type_name: "test_msgs::FallibleEncode".to_string(),
            source: Box::new(IntentionalEncodeFailure),
        }))
    }

    fn serialize(input: Self::Input<'_>) -> Result<Vec<u8>, Self::Error> {
        let _ = input;
        Err(IntentionalEncodeFailure)
    }

    fn serialize_to_buf(input: Self::Input<'_>, _buffer: &mut Vec<u8>) -> Result<(), Self::Error> {
        let _ = input;
        Err(IntentionalEncodeFailure)
    }
}

impl WireDecoder for FailingEncodeCodec {
    type Input<'a> = &'a [u8];
    type Output = FallibleEncode;
    type Error = ros_z::message::CdrError;

    fn deserialize(input: Self::Input<'_>) -> Result<Self::Output, Self::Error> {
        ros_z::SerdeCdrCodec::<FallibleEncode>::deserialize(input)
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn service_timeout_reports_service_name_and_timeout() -> color_eyre::Result<()> {
    color_eyre::install().ok();

    let context = ros_z::context::ContextBuilder::default()
        .disable_multicast_scouting()
        .with_connect_endpoints(std::iter::empty::<&str>())
        .with_listen_endpoints(["tcp/127.0.0.1:0"])
        .build()
        .await?;
    let node = context.create_node("client_node").build().await?;
    let client = node
        .create_service_client::<ros_z::dynamic::GetSchema>("/missing/get_schema")
        .expect("service client factory should succeed")
        .build()
        .await?;

    let request = ros_z::dynamic::GetSchemaRequest {
        root_type_name: "missing::Type".to_string(),
        schema_hash: ros_z::SchemaHash::zero().to_hash_string(),
    };
    let timeout = std::time::Duration::from_millis(10);

    let error = client
        .call_with_timeout_async(&request, timeout)
        .await
        .expect_err("missing service should time out");

    assert!(error.to_string().contains("/missing/get_schema"));
    assert!(error.to_string().contains("timed out"));

    Ok(())
}
