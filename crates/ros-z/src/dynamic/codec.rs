//! Codec implementations for dynamic messages.
//!
//! This module provides `DynamicCdrCodec` which implements the `WireEncoder`
//! and `WireDecoder` traits for `DynamicPayload`. Runtime-typed messages use
//! `Node::dynamic_publisher` and `Node::dynamic_subscriber`.

use std::sync::Arc;

use zenoh_buffers::ZBuf;

use crate::message::{WireDecoder, WireEncoder};

use super::error::DynamicError;
use super::schema::Schema;
use super::value::{DynamicValue, default_for_schema};

#[derive(Clone, Debug, PartialEq)]
pub struct DynamicPayload {
    pub schema: Schema,
    pub value: DynamicValue,
}

impl DynamicPayload {
    pub fn new(schema: Schema, value: DynamicValue) -> Result<Self, DynamicError> {
        schema
            .validate()
            .map_err(|error| DynamicError::SerializationError(error.to_string()))?;
        value.validate_against(&schema)?;
        Ok(Self { schema, value })
    }

    pub fn default_for_schema(schema: Schema) -> Result<Self, DynamicError> {
        let value = default_for_schema(&schema)?;
        Self::new(schema, value)
    }

    pub fn from_struct(message: super::message::DynamicStruct) -> Result<Self, DynamicError> {
        let schema = message.schema_arc();
        Self::new(schema, DynamicValue::Struct(Box::new(message)))
    }
}

/// CDR codec for dynamic root payloads.
///
/// This type implements both `WireEncoder` and `WireDecoder` for
/// [`DynamicPayload`]. Typed messages select custom codecs through their
/// `Message::Codec` associated type; runtime-typed messages use
/// `Node::dynamic_publisher` and `Node::dynamic_subscriber`.
pub struct DynamicCdrCodec;

impl DynamicCdrCodec {
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
