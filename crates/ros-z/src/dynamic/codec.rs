//! Codec implementations for dynamic messages.
//!
//! This module provides `DynamicCdrCodec` which implements the `WireEncoder`
//! and `WireDecoder` traits, allowing `DynamicMessage` to be used with
//! the standard `Publisher`/`Subscriber` infrastructure.

use std::sync::Arc;

use zenoh_buffers::ZBuf;

use crate::msg::{WireDecoder, WireEncoder};

use super::error::DynamicError;
use super::message::DynamicMessage;
use super::schema::MessageSchema;

/// CDR codec for `DynamicMessage`.
///
/// This type implements both `WireEncoder` and `WireDecoder`, enabling
/// `DynamicMessage` to work with the standard pub/sub infrastructure.
///
/// # Example
///
/// ```ignore
/// use ros_z::dynamic::{DynamicCdrCodec, DynamicMessage, MessageSchema};
/// use ros_z::pubsub::{Publisher, Subscriber};
///
/// // Publisher - schema is embedded in DynamicMessage
/// let publisher: Publisher<DynamicMessage, DynamicCdrCodec> = node
///     .publisher("/topic")
///     .codec::<DynamicCdrCodec>()
///     .build()
///     .await?;
///
/// // Subscriber - schema must be provided via dyn_schema()
/// let subscriber: Subscriber<DynamicMessage, DynamicCdrCodec> = node
///     .subscriber("/topic")
///     .codec::<DynamicCdrCodec>()
///     .dyn_schema(schema)
///     .build()
///     .await?;
/// ```
pub struct DynamicCdrCodec;

impl DynamicCdrCodec {
    fn ensure_schema_matches(
        input: &DynamicMessage,
        schema: &Arc<MessageSchema>,
    ) -> Result<(), DynamicError> {
        if input.schema() == schema.as_ref() {
            return Ok(());
        }

        Err(DynamicError::SerializationError(format!(
            "schema mismatch: message schema '{}' does not match supplied schema '{}'",
            input.schema().type_name_str(),
            schema.type_name_str()
        )))
    }

    pub fn encode(
        input: &DynamicMessage,
        schema: &Arc<MessageSchema>,
    ) -> Result<crate::msg::EncodedMessage, DynamicError> {
        Self::ensure_schema_matches(input, schema)?;
        Ok(crate::msg::EncodedMessage {
            payload: input.to_cdr_zbuf()?,
            encoding: crate::encoding::Encoding::cdr(),
        })
    }

    pub fn decode(
        bytes: &[u8],
        schema: &Arc<MessageSchema>,
    ) -> Result<DynamicMessage, DynamicError> {
        DynamicMessage::from_cdr(bytes, schema)
    }

    pub fn encoded_size_hint(input: &DynamicMessage, schema: &Arc<MessageSchema>) -> usize {
        // Dynamic fields are variable-sized; use the supplied schema explicitly as
        // a conservative floor while keeping the infallible Phase 1 API shape.
        4 + schema.fields().len().max(input.schema().fields().len()) * 16
    }

    pub fn try_serialize_to_zbuf(input: &DynamicMessage) -> Result<ZBuf, DynamicError> {
        input.to_cdr_zbuf()
    }

    pub fn try_serialize(input: &DynamicMessage) -> Result<Vec<u8>, DynamicError> {
        input.to_cdr()
    }

    pub fn try_serialize_to_buf(
        input: &DynamicMessage,
        buffer: &mut Vec<u8>,
    ) -> Result<(), DynamicError> {
        let data = input.to_cdr()?;
        buffer.clear();
        buffer.extend(data);
        Ok(())
    }
}

impl WireEncoder for DynamicCdrCodec {
    type Input<'a> = &'a DynamicMessage;

    fn serialize_to_zbuf(input: &DynamicMessage) -> ZBuf {
        input
            .to_cdr_zbuf()
            .expect("DynamicMessage CDR serialization failed")
    }

    fn serialize_to_zbuf_with_hint(input: &DynamicMessage, _capacity_hint: usize) -> ZBuf {
        // DynamicMessage doesn't use capacity hints (it has its own serialization path)
        Self::serialize_to_zbuf(input)
    }

    fn serialized_size_hint(input: &DynamicMessage) -> usize {
        let _ = input;
        256
    }

    fn serialize_to_shm(
        input: &DynamicMessage,
        _estimated_size: usize,
        provider: &zenoh::shm::ShmProvider<zenoh::shm::PosixShmProviderBackend>,
    ) -> zenoh::Result<(ZBuf, usize)> {
        // DynamicMessage uses primitives-based serialization, not serde
        // So we serialize to Vec first, then copy to SHM
        let data = input.to_cdr().map_err(|e| {
            zenoh::Error::from(format!("DynamicMessage serialization failed: {}", e))
        })?;
        let actual_size = data.len();

        use zenoh::Wait;
        use zenoh::shm::{BlockOn, GarbageCollect};

        let mut shm_buf = provider
            .alloc(actual_size)
            .with_policy::<BlockOn<GarbageCollect>>()
            .wait()
            .map_err(|e| zenoh::Error::from(format!("SHM allocation failed: {}", e)))?;

        shm_buf[0..actual_size].copy_from_slice(&data);

        Ok((ZBuf::from(shm_buf), actual_size))
    }

    fn serialize(input: &DynamicMessage) -> Vec<u8> {
        input
            .to_cdr()
            .expect("DynamicMessage CDR serialization failed")
    }

    fn serialize_to_buf(input: &DynamicMessage, buffer: &mut Vec<u8>) {
        buffer.clear();
        buffer.extend(
            input
                .to_cdr()
                .expect("DynamicMessage CDR serialization failed"),
        );
    }
}

impl WireDecoder for DynamicCdrCodec {
    type Input<'a> = (&'a [u8], &'a Arc<MessageSchema>);
    type Output = DynamicMessage;
    type Error = DynamicError;

    fn deserialize(input: Self::Input<'_>) -> Result<DynamicMessage, DynamicError> {
        let (bytes, schema) = input;
        DynamicMessage::from_cdr(bytes, schema)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dynamic::schema::{FieldType, MessageSchema};
    use crate::dynamic::value::DynamicValue;
    use zenoh_buffers::buffer::Buffer;

    fn create_point_schema() -> Arc<MessageSchema> {
        MessageSchema::builder("geometry_msgs::Point")
            .field("x", FieldType::Float64)
            .field("y", FieldType::Float64)
            .field("z", FieldType::Float64)
            .build()
            .unwrap()
    }

    #[test]
    fn test_serialize_to_zbuf() {
        let schema = create_point_schema();
        let mut message = DynamicMessage::new(&schema);
        message.set("x", 1.0f64).unwrap();
        message.set("y", 2.0f64).unwrap();
        message.set("z", 3.0f64).unwrap();

        let zbuf = DynamicCdrCodec::serialize_to_zbuf(&message);
        assert!(zbuf.len() > 0);
    }

    #[test]
    fn test_serialize_deserialize_roundtrip() {
        let schema = create_point_schema();
        let mut message = DynamicMessage::new(&schema);
        message.set("x", 1.5f64).unwrap();
        message.set("y", 2.5f64).unwrap();
        message.set("z", 3.5f64).unwrap();

        // Serialize
        let bytes = DynamicCdrCodec::serialize(&message);

        // Deserialize
        let deserialized = DynamicCdrCodec::deserialize((&bytes, &schema)).unwrap();

        assert_eq!(deserialized.get::<f64>("x").unwrap(), 1.5);
        assert_eq!(deserialized.get::<f64>("y").unwrap(), 2.5);
        assert_eq!(deserialized.get::<f64>("z").unwrap(), 3.5);
    }

    #[test]
    fn test_serialize_to_buf() {
        let schema = create_point_schema();
        let mut message = DynamicMessage::new(&schema);
        message.set("x", 1.0f64).unwrap();
        message.set("y", 2.0f64).unwrap();
        message.set("z", 3.0f64).unwrap();

        let mut buffer = Vec::new();
        DynamicCdrCodec::serialize_to_buf(&message, &mut buffer);

        // Should match serialize() output
        let direct = DynamicCdrCodec::serialize(&message);
        assert_eq!(buffer, direct);
    }

    #[test]
    fn try_serialize_reports_invalid_dynamic_message() {
        let schema = MessageSchema::builder("test_msgs::Invalid")
            .field("count", FieldType::Uint32)
            .build()
            .unwrap();
        let mut message = DynamicMessage::new(&schema);
        message
            .set_dynamic("count", DynamicValue::String("not an integer".into()))
            .unwrap();

        let error = DynamicCdrCodec::try_serialize(&message)
            .expect_err("invalid dynamic message should be returned");

        assert!(error.to_string().contains("Type mismatch"));

        let error = DynamicCdrCodec::try_serialize_to_zbuf(&message)
            .expect_err("invalid dynamic message should be returned");

        assert!(error.to_string().contains("Type mismatch"));

        let mut buffer = vec![1, 2, 3];
        let error = DynamicCdrCodec::try_serialize_to_buf(&message, &mut buffer)
            .expect_err("invalid dynamic message should be returned");

        assert!(error.to_string().contains("Type mismatch"));
        assert_eq!(buffer, vec![1, 2, 3]);
    }

    #[test]
    fn dynamic_cdr_codec_rejects_schema_mismatch_on_encode() {
        let schema = create_point_schema();
        let other_schema = MessageSchema::builder("geometry_msgs::Vector3")
            .field("x", FieldType::Float64)
            .field("y", FieldType::Float64)
            .field("z", FieldType::Float64)
            .build()
            .unwrap();
        let mut message = DynamicMessage::new(&schema);
        message.set("x", 1.0f64).unwrap();
        message.set("y", 2.0f64).unwrap();
        message.set("z", 3.0f64).unwrap();

        let error = match DynamicCdrCodec::encode(&message, &other_schema) {
            Ok(_) => panic!("schema mismatch should fail"),
            Err(error) => error,
        };

        assert!(error.to_string().contains("schema mismatch"));
    }
}
