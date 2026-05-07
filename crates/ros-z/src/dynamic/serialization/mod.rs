//! Serialization support for dynamic messages.
//!
//! This module provides serialization and deserialization for dynamic
//! messages using CDR.

mod cdr;

pub use cdr::{
    deserialize_cdr, deserialize_cdr_value, serialize_cdr, serialize_cdr_to_zbuf,
    serialize_cdr_value,
};

use zenoh_buffers::ZBuf;

use super::error::DynamicError;
use super::message::DynamicStruct;
use super::schema::Schema;

/// Supported serialization formats for dynamic messages.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SerializationFormat {
    #[default]
    Cdr,
}

/// CDR encapsulation header for little-endian encoding.
pub const CDR_HEADER_LE: [u8; 4] = [0x00, 0x01, 0x00, 0x00];

impl DynamicStruct {
    /// Serialize the message to bytes using the specified format.
    pub fn serialize(&self, format: SerializationFormat) -> Result<Vec<u8>, DynamicError> {
        match format {
            SerializationFormat::Cdr => serialize_cdr(self),
        }
    }

    /// Serialize the message to an existing buffer.
    pub fn serialize_to_buf(
        &self,
        format: SerializationFormat,
        buffer: &mut Vec<u8>,
    ) -> Result<(), DynamicError> {
        buffer.clear();
        let data = self.serialize(format)?;
        buffer.extend_from_slice(&data);
        Ok(())
    }

    /// Serialize the message to a ZBuf (zero-copy where possible).
    pub fn serialize_to_zbuf(&self, format: SerializationFormat) -> Result<ZBuf, DynamicError> {
        match format {
            SerializationFormat::Cdr => serialize_cdr_to_zbuf(self),
        }
    }

    /// Deserialize a message from bytes.
    pub fn deserialize(
        data: &[u8],
        schema: &Schema,
        format: SerializationFormat,
    ) -> Result<Self, DynamicError> {
        match format {
            SerializationFormat::Cdr => deserialize_cdr(data, schema),
        }
    }

    // Convenience methods with default CDR format

    /// Serialize to CDR format.
    pub fn to_cdr(&self) -> Result<Vec<u8>, DynamicError> {
        serialize_cdr(self)
    }

    /// Serialize to CDR format as ZBuf.
    pub fn to_cdr_zbuf(&self) -> Result<ZBuf, DynamicError> {
        serialize_cdr_to_zbuf(self)
    }

    /// Deserialize from CDR format.
    pub fn from_cdr(data: &[u8], schema: &Schema) -> Result<Self, DynamicError> {
        deserialize_cdr(data, schema)
    }
}
