//! Codec implementations for dynamic messages.
//!
//! This module provides `DynamicCdrCodec` which implements the `WireEncoder`
//! and `WireDecoder` traits, allowing `DynamicStruct` to be used with
//! the standard `Publisher`/`Subscriber` infrastructure.

use std::sync::Arc;

use zenoh_buffers::ZBuf;

use crate::msg::{WireDecoder, WireEncoder};

use super::error::DynamicError;
use super::message::DynamicStruct;
use super::schema::{Schema, TypeShape};
use super::value::DynamicValue;

#[derive(Clone, Debug, PartialEq)]
pub struct DynamicPayload {
    pub schema: Schema,
    pub value: DynamicValue,
}

impl DynamicPayload {
    pub fn new(schema: Schema, value: DynamicValue) -> Result<Self, DynamicError> {
        value.validate_against(&schema)?;
        Ok(Self { schema, value })
    }

    pub fn from_struct(message: DynamicStruct) -> Result<Self, DynamicError> {
        let schema = message.schema_arc();
        Self::new(schema, DynamicValue::Struct(Box::new(message)))
    }
}

/// CDR codec for dynamic root payloads.
///
/// This type implements both `WireEncoder` and `WireDecoder`, enabling
/// `DynamicStruct` to work with the standard pub/sub infrastructure.
///
/// # Example
///
/// ```ignore
/// use ros_z::dynamic::{DynamicCdrCodec, DynamicPayload};
/// use ros_z::pubsub::{Publisher, Subscriber};
///
/// // Publisher - schema is carried in DynamicPayload
/// let publisher: Publisher<DynamicPayload, DynamicCdrCodec> = node
///     .publisher("/topic")
///     .codec::<DynamicCdrCodec>()
///     .build()
///     .await?;
///
/// // Subscriber - schema is provided to the decoder path
/// let subscriber: Subscriber<DynamicPayload, DynamicCdrCodec> = node
///     .subscriber("/topic")
///     .codec::<DynamicCdrCodec>()
///     .build()
///     .await?;
/// ```
pub struct DynamicCdrCodec;

impl DynamicCdrCodec {
    fn ensure_message_schema_matches(
        input: &DynamicStruct,
        schema: &Schema,
    ) -> Result<(), DynamicError> {
        if input.schema() == schema.as_ref() {
            return Ok(());
        }

        Err(DynamicError::SerializationError(format!(
            "schema mismatch: message schema '{}' does not match supplied schema '{}'",
            schema_name(input.schema()),
            schema_name(schema.as_ref())
        )))
    }

    pub fn encode_message(
        input: &DynamicStruct,
        schema: &Schema,
    ) -> Result<crate::msg::EncodedMessage, DynamicError> {
        Self::ensure_message_schema_matches(input, schema)?;
        Ok(crate::msg::EncodedMessage {
            payload: input.to_cdr_zbuf()?,
            encoding: crate::encoding::Encoding::cdr(),
        })
    }

    pub fn decode_message(bytes: &[u8], schema: &Schema) -> Result<DynamicStruct, DynamicError> {
        DynamicStruct::from_cdr(bytes, schema)
    }

    pub fn encode(input: &DynamicPayload) -> Result<crate::msg::EncodedMessage, DynamicError> {
        Ok(crate::msg::EncodedMessage {
            payload: Self::try_serialize_payload_to_zbuf(input)?,
            encoding: crate::encoding::Encoding::cdr(),
        })
    }

    pub fn decode(bytes: &[u8], schema: &Schema) -> Result<DynamicPayload, DynamicError> {
        let value = super::serialization::deserialize_cdr_value(bytes, schema)?;
        DynamicPayload::new(Arc::clone(schema), value)
    }

    pub fn try_serialize_payload_to_zbuf(input: &DynamicPayload) -> Result<ZBuf, DynamicError> {
        let bytes = Self::try_serialize_payload(input)?;
        Ok(ZBuf::from(bytes))
    }

    pub fn try_serialize_payload(input: &DynamicPayload) -> Result<Vec<u8>, DynamicError> {
        super::serialization::serialize_cdr_value(&input.schema, &input.value)
    }

    pub fn encoded_size_hint(input: &DynamicStruct, schema: &Schema) -> usize {
        // Dynamic fields are variable-sized; use the supplied schema explicitly as
        // a conservative floor while keeping the infallible Phase 1 API shape.
        4 + field_count(schema.as_ref()).max(field_count(input.schema())) * 16
    }

    pub fn try_serialize_to_zbuf(input: &DynamicStruct) -> Result<ZBuf, DynamicError> {
        input.to_cdr_zbuf()
    }

    pub fn try_serialize(input: &DynamicStruct) -> Result<Vec<u8>, DynamicError> {
        input.to_cdr()
    }

    pub fn try_serialize_to_buf(
        input: &DynamicStruct,
        buffer: &mut Vec<u8>,
    ) -> Result<(), DynamicError> {
        let data = input.to_cdr()?;
        buffer.clear();
        buffer.extend(data);
        Ok(())
    }
}

fn schema_name(schema: &TypeShape) -> &str {
    match schema {
        TypeShape::Struct { name, .. } | TypeShape::Enum { name, .. } => name.as_str(),
        TypeShape::Primitive(_) => "<primitive>",
        TypeShape::String => "<string>",
        TypeShape::Optional(_) => "<optional>",
        TypeShape::Sequence { .. } => "<sequence>",
        TypeShape::Map { .. } => "<map>",
    }
}

fn field_count(schema: &TypeShape) -> usize {
    match schema {
        TypeShape::Struct { fields, .. } => fields.len(),
        _ => 0,
    }
}

impl WireEncoder for DynamicCdrCodec {
    type Input<'a> = &'a DynamicPayload;

    fn serialize_to_zbuf(input: &DynamicPayload) -> ZBuf {
        Self::try_serialize_payload_to_zbuf(input).expect("DynamicPayload CDR serialization failed")
    }

    fn serialize_to_zbuf_with_hint(input: &DynamicPayload, _capacity_hint: usize) -> ZBuf {
        // DynamicPayload doesn't use capacity hints (it has its own serialization path)
        Self::serialize_to_zbuf(input)
    }

    fn serialized_size_hint(input: &DynamicPayload) -> usize {
        let _ = input;
        256
    }

    fn serialize_to_shm(
        input: &DynamicPayload,
        _estimated_size: usize,
        provider: &zenoh::shm::ShmProvider<zenoh::shm::PosixShmProviderBackend>,
    ) -> zenoh::Result<(ZBuf, usize)> {
        // DynamicPayload uses primitives-based serialization, not serde
        // So we serialize to Vec first, then copy to SHM
        let data = Self::try_serialize_payload(input).map_err(|e| {
            zenoh::Error::from(format!("DynamicPayload serialization failed: {}", e))
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

    fn serialize(input: &DynamicPayload) -> Vec<u8> {
        Self::try_serialize_payload(input).expect("DynamicPayload CDR serialization failed")
    }

    fn serialize_to_buf(input: &DynamicPayload, buffer: &mut Vec<u8>) {
        buffer.clear();
        buffer.extend(
            Self::try_serialize_payload(input).expect("DynamicPayload CDR serialization failed"),
        );
    }
}

impl WireDecoder for DynamicCdrCodec {
    type Input<'a> = (&'a [u8], &'a Schema);
    type Output = DynamicPayload;
    type Error = DynamicError;

    fn deserialize(input: Self::Input<'_>) -> Result<DynamicPayload, DynamicError> {
        let (bytes, schema) = input;
        Self::decode(bytes, schema)
    }
}
