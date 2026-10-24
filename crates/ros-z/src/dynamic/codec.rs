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
            .map_err(|error| DynamicError::schema("creating dynamic payload", error))?;
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
        Ok(DynamicPayload {
            schema: Arc::clone(schema),
            value,
        })
    }

    // Subscriber construction already validates this immutable schema.
    pub(crate) fn decode_payload(
        payload: &zenoh::bytes::ZBytes,
        schema: &Schema,
    ) -> Result<DynamicPayload, DynamicError> {
        let buffer: ZBuf = payload.clone().into();
        let reader =
            ros_z_cdr::CdrReader::from_zbuf(&buffer).map_err(DynamicError::deserialization)?;
        let value = super::serialization::deserialize_cdr_reader(reader, schema)?;
        Ok(DynamicPayload {
            schema: Arc::clone(schema),
            value,
        })
    }

    pub fn try_serialize_payload_to_zbuf(input: &DynamicPayload) -> Result<ZBuf, DynamicError> {
        let mut buffer = ros_z_cdr::ZBufWriter::with_capacity(256);
        super::serialization::serialize_cdr_value_into(&input.schema, &input.value, &mut buffer)?;
        Ok(buffer.into_zbuf())
    }

    pub fn try_serialize_payload(input: &DynamicPayload) -> Result<Vec<u8>, DynamicError> {
        super::serialization::serialize_cdr_value(&input.schema, &input.value)
    }
}

impl WireEncoder for DynamicCdrCodec {
    type Input<'a> = &'a DynamicPayload;
    type Error = DynamicError;

    fn serialize_to_zbuf(input: &DynamicPayload) -> Result<ZBuf, Self::Error> {
        Self::try_serialize_payload_to_zbuf(input)
    }

    fn serialize_to_zbuf_with_hint(
        input: &DynamicPayload,
        capacity_hint: usize,
    ) -> Result<ZBuf, Self::Error> {
        let mut buffer = ros_z_cdr::ZBufWriter::with_capacity(capacity_hint);
        super::serialization::serialize_cdr_value_into(&input.schema, &input.value, &mut buffer)?;
        Ok(buffer.into_zbuf())
    }

    fn serialized_size_hint(input: &DynamicPayload) -> usize {
        let mut counter = crate::native_cdr::SizeCounter::default();
        super::serialization::write_cdr_value(&input.schema, &input.value, &mut counter)
            .and_then(|()| {
                counter
                    .finish()
                    .map_err(|error| DynamicError::SerializationError(error.to_string()))
            })
            .unwrap_or(0)
    }

    fn serialize_to_shm(
        input: &DynamicPayload,
        estimated_size: usize,
        provider: &zenoh::shm::ShmProvider<zenoh::shm::PosixShmProviderBackend>,
    ) -> crate::Result<(ZBuf, usize)> {
        let mut writer =
            crate::shm::ShmWriter::new(provider, estimated_size.max(4)).or_else(|error| {
                let size = Self::serialized_size_hint(input);
                if size == 0 || size >= estimated_size {
                    return Err(error);
                }
                crate::shm::ShmWriter::new(provider, size)
            })?;
        super::serialization::serialize_cdr_value_into(&input.schema, &input.value, &mut writer)
            .map_err(|source| crate::Error::encode("ros_z::dynamic::DynamicPayload", source))?;
        if writer.overflowed() {
            let size = writer.position();
            drop(writer);
            writer = crate::shm::ShmWriter::new(provider, size)?;
            super::serialization::write_cdr_value(&input.schema, &input.value, &mut writer)
                .map_err(|source| crate::Error::encode("ros_z::dynamic::DynamicPayload", source))?;
        }
        let actual_size = writer.position();
        Ok((writer.into_zbuf()?, actual_size))
    }

    fn serialize(input: &DynamicPayload) -> Result<Vec<u8>, Self::Error> {
        Self::try_serialize_payload(input)
    }

    fn serialize_to_buf(input: &DynamicPayload, buffer: &mut Vec<u8>) -> Result<(), Self::Error> {
        buffer.clear();
        super::serialization::serialize_cdr_value_into(&input.schema, &input.value, buffer)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Message;
    use zenoh_buffers::{ZSlice, buffer::SplitBuffer};

    #[test]
    fn dynamic_bytes_share_storage_and_reuse_output() {
        let bytes = ZSlice::from(vec![7; 8192]);
        let pointer = bytes.as_ptr();
        let schema = Arc::new(ZSlice::schema());
        let input = DynamicPayload::new(schema.clone(), DynamicValue::Bytes(bytes)).unwrap();
        let wire = DynamicCdrCodec::serialize(&input).unwrap();
        assert_eq!(DynamicCdrCodec::serialized_size_hint(&input), wire.len());

        let mut output = Vec::with_capacity(wire.len() + 128);
        let allocation = output.as_ptr();
        DynamicCdrCodec::serialize_to_buf(&input, &mut output).unwrap();
        assert_eq!(output.as_ptr(), allocation);
        assert_eq!(output, wire);

        let payload = DynamicCdrCodec::serialize_to_zbuf(&input).unwrap().into();
        let decoded = DynamicCdrCodec::decode_payload(&payload, &schema).unwrap();
        let DynamicValue::Bytes(bytes) = &decoded.value else {
            panic!("expected bytes")
        };
        assert_eq!(bytes.as_ptr(), pointer);
        drop(input);
        drop(payload);
        assert_eq!(bytes.as_ref(), &[7; 8192]);

        let mut fragmented = ZBuf::empty();
        for part in wire.chunks(3) {
            fragmented.push_zslice(part.to_vec().into());
        }
        let decoded_fragmented =
            DynamicCdrCodec::decode_payload(&fragmented.into(), &schema).unwrap();
        assert_eq!(decoded_fragmented, decoded);

        let provider = crate::shm::ShmProviderBuilder::new(64 * 1024)
            .build()
            .unwrap();
        for estimate in [1, wire.len(), wire.len() + 128] {
            let (buffer, size) =
                DynamicCdrCodec::serialize_to_shm(&decoded, estimate, &provider).unwrap();
            assert_eq!(size, wire.len());
            assert_eq!(buffer.contiguous().as_ref(), wire);
        }
        assert!(DynamicCdrCodec::decode(&wire[..wire.len() - 1], &schema).is_err());
    }
}
