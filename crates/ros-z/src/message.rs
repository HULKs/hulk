use byteorder::LittleEndian;
use ros_z_cdr::{CdrBuffer, SerdeCdrSerializer, ZBufWriter};
use ros_z_schema::{
    PrimitiveTypeDef, SchemaBundle, SchemaError, SchemaHash, SequenceLengthDef, TypeDef, TypeName,
};
use serde::{Serialize, de::DeserializeOwned};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::Hash;
use std::marker::PhantomData;
use std::net::SocketAddr;
use std::ops::{Range, RangeInclusive};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use zenoh::shm::{PosixShmProviderBackend, ShmProvider};
use zenoh_buffers::ZBuf;

use crate::entity::TypeInfo;
use crate::schema::{MessageSchema, SchemaBuilder};
use crate::shm::ShmWriter;

/// Error returned when CDR bytes cannot be decoded into the requested type.
#[derive(Debug)]
pub struct CdrError(String);

impl std::fmt::Display for CdrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CDR deserialization error: {}", self.0)
    }
}

impl std::error::Error for CdrError {}

/// Codec-side encoder used by publishers and prepared publications.
pub trait WireEncoder {
    type Input<'a>
    where
        Self: 'a;

    /// Serialize directly to a ZBuf for zero-copy publishing.
    ///
    /// This is the primary serialization method that returns a ZBuf,
    /// optimized for Zenoh publishing without intermediate copies.
    ///
    /// Uses a fixed 256-byte initial capacity. For better performance with
    /// large messages, use `serialize_to_zbuf_with_hint()` when the caller has
    /// a better capacity estimate.
    fn serialize_to_zbuf(input: Self::Input<'_>) -> ZBuf;

    /// Serialize to ZBuf with a capacity hint for optimal allocation.
    ///
    /// This method uses the provided capacity hint to pre-allocate the buffer,
    /// reducing or eliminating reallocations for large messages.
    ///
    /// # Arguments
    ///
    /// * `input` - The message to serialize
    /// * `capacity_hint` - Expected serialized size in bytes
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ros_z::message::{WireEncoder, SerdeCdrCodec};
    /// use serde::Serialize;
    ///
    /// #[derive(Serialize)]
    /// struct LargeMsg { data: Vec<u8> }
    ///
    /// let message = LargeMsg { data: vec![0; 1_000_000] };
    /// let hint = 4 + 4 + 1_000_000;  // header + length + data
    /// let zbuf = SerdeCdrCodec::<LargeMsg>::serialize_to_zbuf_with_hint(&message, hint);
    /// ```
    fn serialize_to_zbuf_with_hint(input: Self::Input<'_>, capacity_hint: usize) -> ZBuf;

    /// Return a conservative serialized-size estimate for buffer preallocation.
    fn serialized_size_hint(_input: Self::Input<'_>) -> usize {
        256
    }

    /// Serialize directly to shared memory for zero-copy publishing.
    ///
    /// This method serializes the message directly into a pre-allocated SHM buffer,
    /// avoiding any intermediate copies.
    ///
    /// # Arguments
    ///
    /// * `input` - The message to serialize
    /// * `estimated_size` - Conservative upper bound on serialized size
    /// * `provider` - SHM provider for buffer allocation
    ///
    /// # Returns
    ///
    /// A tuple of (ZBuf, actual_size) where:
    /// - ZBuf is backed by SHM
    /// - actual_size is the exact number of bytes written
    ///
    /// # Errors
    ///
    /// Returns an error if SHM allocation fails.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ros_z::message::{WireEncoder, SerdeCdrCodec};
    /// use ros_z::shm::ShmProviderBuilder;
    /// use serde::Serialize;
    ///
    /// # fn main() -> zenoh::Result<()> {
    /// #[derive(Serialize)]
    /// struct MyMsg { value: u32 }
    ///
    /// let message = MyMsg { value: 42 };
    /// let provider = ShmProviderBuilder::new(1024 * 1024).build()?;
    ///
    /// let (zbuf, size) = SerdeCdrCodec::<MyMsg>::serialize_to_shm(&message, 128, &provider)?;
    /// println!("Serialized {} bytes to SHM", size);
    /// # Ok(())
    /// # }
    /// ```
    fn serialize_to_shm(
        input: Self::Input<'_>,
        estimated_size: usize,
        provider: &ShmProvider<PosixShmProviderBackend>,
    ) -> zenoh::Result<(ZBuf, usize)>;

    /// Serialize to an existing buffer, returning the result as ZBuf.
    ///
    /// This variant allows buffer reuse for reduced allocations.
    /// The buffer is cleared and reused, then wrapped in a ZBuf.
    fn serialize_to_zbuf_reuse(input: Self::Input<'_>, buffer: &mut Vec<u8>) -> ZBuf {
        Self::serialize_to_buf(input, buffer);
        // Take ownership of the buffer contents, leaving an empty Vec
        ZBuf::from(std::mem::take(buffer))
    }

    /// Serialize to an owned byte vector for callers that need contiguous bytes.
    ///
    /// Prefer `serialize_to_zbuf()` for zero-copy publishing.
    fn serialize(input: Self::Input<'_>) -> Vec<u8> {
        let mut buffer = Vec::new();
        Self::serialize_to_buf(input, &mut buffer);
        buffer
    }

    /// Serialize to an existing buffer, reusing its allocation.
    ///
    /// The buffer is cleared before writing. Implementations should
    /// write directly to the buffer for optimal performance.
    fn serialize_to_buf(input: Self::Input<'_>, buffer: &mut Vec<u8>);
}

/// Typed message contract for ros-z publishers, subscribers, services, and schemas.
pub trait Message: MessageSchema + Send + Sync + Sized + 'static {
    /// Codec used to encode and decode this message type.
    type Codec: for<'a> WireEncoder<Input<'a> = &'a Self>
        + for<'a> WireDecoder<Input<'a> = &'a [u8], Output = Self>;

    /// Stable fully qualified type name advertised in graph metadata.
    fn type_name() -> String;
    /// Runtime schema used for discovery and dynamic tooling.
    fn schema() -> Result<SchemaBundle, SchemaError> {
        crate::schema::schema_for::<Self>()
    }

    /// Stable hash derived from [`Message::schema`].
    fn schema_hash() -> Result<SchemaHash, SchemaError> {
        Ok(ros_z_schema::compute_hash(&Self::schema()?))
    }

    /// Type name plus schema hash advertised for this message.
    fn type_info() -> Result<TypeInfo, SchemaError> {
        Ok(TypeInfo::with_hash(
            &Self::type_name(),
            Self::schema_hash()?,
        ))
    }
}

macro_rules! impl_primitive_message {
    ($ty:ty, $name:literal, $primitive:ident) => {
        impl MessageSchema for $ty {
            fn build_schema(_builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
                Ok(TypeDef::Primitive(PrimitiveTypeDef::$primitive))
            }
        }

        impl Message for $ty {
            type Codec = SerdeCdrCodec<Self>;

            fn type_name() -> String {
                $name.to_string()
            }
        }
    };
}

impl_primitive_message!(bool, "bool", Bool);
impl_primitive_message!(i8, "i8", I8);
impl_primitive_message!(u8, "u8", U8);
impl_primitive_message!(i16, "i16", I16);
impl_primitive_message!(u16, "u16", U16);
impl_primitive_message!(i32, "i32", I32);
impl_primitive_message!(u32, "u32", U32);
impl_primitive_message!(i64, "i64", I64);
impl_primitive_message!(u64, "u64", U64);
impl_primitive_message!(usize, "usize", U64);
impl_primitive_message!(f32, "f32", F32);
impl_primitive_message!(f64, "f64", F64);

impl<T> Message for Box<T>
where
    T: Message + Serialize + DeserializeOwned,
{
    type Codec = SerdeCdrCodec<Self>;

    fn type_name() -> String {
        format!("Box<{}>", T::type_name())
    }
}

impl<T> MessageSchema for Box<T>
where
    T: MessageSchema,
{
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        T::build_schema(builder)
    }
}

impl<T> Message for Arc<T>
where
    T: Message + Serialize + DeserializeOwned,
{
    type Codec = SerdeCdrCodec<Self>;

    fn type_name() -> String {
        format!("Arc<{}>", T::type_name())
    }
}

impl<T> MessageSchema for Arc<T>
where
    T: MessageSchema,
{
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        T::build_schema(builder)
    }
}

impl Message for Duration {
    type Codec = SerdeCdrCodec<Self>;

    fn type_name() -> String {
        "std::time::Duration".to_string()
    }
}

impl MessageSchema for Duration {
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        builder.define_message_struct::<Self>(|fields| {
            fields.field::<u64>("secs")?;
            fields.field::<u32>("nanos")?;
            Ok(())
        })
    }
}

impl Message for SystemTime {
    type Codec = SerdeCdrCodec<Self>;

    fn type_name() -> String {
        "std::time::SystemTime".to_string()
    }
}

impl MessageSchema for SystemTime {
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        builder.define_message_struct::<Self>(|fields| {
            fields.field::<u64>("secs_since_epoch")?;
            fields.field::<u32>("nanos_since_epoch")?;
            Ok(())
        })
    }
}

impl<T> Message for Range<T>
where
    T: Message + Serialize + DeserializeOwned,
{
    type Codec = SerdeCdrCodec<Self>;

    fn type_name() -> String {
        format!("Range<{}>", T::type_name())
    }
}

impl<T> MessageSchema for Range<T>
where
    T: Message,
{
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        let name = TypeName::new(format!("Range<{}>", T::type_name()))?;
        builder.define_struct(name, |fields| {
            fields.field::<T>("start")?;
            fields.field::<T>("end")?;
            Ok(())
        })
    }
}

impl<T> Message for RangeInclusive<T>
where
    T: Message + Serialize + DeserializeOwned,
{
    type Codec = SerdeCdrCodec<Self>;

    fn type_name() -> String {
        format!("RangeInclusive<{}>", T::type_name())
    }
}

impl<T> MessageSchema for RangeInclusive<T>
where
    T: Message,
{
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        let name = TypeName::new(format!("RangeInclusive<{}>", T::type_name()))?;
        builder.define_struct(name, |fields| {
            fields.field::<T>("start")?;
            fields.field::<T>("end")?;
            Ok(())
        })
    }
}

impl Message for SocketAddr {
    type Codec = SerdeCdrCodec<Self>;

    fn type_name() -> String {
        "std::net::SocketAddr".to_string()
    }
}

impl MessageSchema for SocketAddr {
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        let v4_name = TypeName::new("std::net::SocketAddrV4")?;
        let v4 = builder.define_struct(v4_name, |fields| {
            let element = fields.shape::<u8>()?;
            fields.field_with_shape(
                "ip",
                TypeDef::Sequence {
                    element: Box::new(element),
                    length: SequenceLengthDef::Fixed(4),
                },
            );
            fields.field::<u16>("port")?;
            Ok(())
        })?;

        let v6_name = TypeName::new("std::net::SocketAddrV6")?;
        let v6 = builder.define_struct(v6_name, |fields| {
            let element = fields.shape::<u8>()?;
            fields.field_with_shape(
                "ip",
                TypeDef::Sequence {
                    element: Box::new(element),
                    length: SequenceLengthDef::Fixed(16),
                },
            );
            fields.field::<u16>("port")?;
            Ok(())
        })?;

        builder.define_message_enum::<Self>(|variants| {
            variants.newtype_with_shape("V4", v4);
            variants.newtype_with_shape("V6", v6);
            Ok(())
        })
    }
}

impl Message for String {
    type Codec = SerdeCdrCodec<Self>;

    fn type_name() -> String {
        "String".to_string()
    }
}

impl MessageSchema for String {
    fn build_schema(_builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        Ok(TypeDef::String)
    }
}

impl<T> Message for Option<T>
where
    T: Message + Serialize + DeserializeOwned,
{
    type Codec = SerdeCdrCodec<Self>;

    fn type_name() -> String {
        format!("Option<{}>", T::type_name())
    }
}

impl<T> MessageSchema for Option<T>
where
    T: MessageSchema,
{
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        Ok(TypeDef::Optional(Box::new(T::build_schema(builder)?)))
    }
}

impl<T> Message for Vec<T>
where
    T: Message + Serialize + DeserializeOwned,
{
    type Codec = SerdeCdrCodec<Self>;

    fn type_name() -> String {
        format!("Vec<{}>", T::type_name())
    }
}

impl<T> MessageSchema for Vec<T>
where
    T: MessageSchema,
{
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        Ok(TypeDef::Sequence {
            element: Box::new(T::build_schema(builder)?),
            length: SequenceLengthDef::Dynamic,
        })
    }
}

impl<T, const N: usize> Message for [T; N]
where
    T: Message + Serialize + DeserializeOwned,
    [T; N]: Serialize + DeserializeOwned,
{
    type Codec = SerdeCdrCodec<Self>;

    fn type_name() -> String {
        format!("[{};{}]", T::type_name(), N)
    }
}

impl<T, const N: usize> MessageSchema for [T; N]
where
    T: MessageSchema,
{
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        Ok(TypeDef::Sequence {
            element: Box::new(T::build_schema(builder)?),
            length: SequenceLengthDef::Fixed(N),
        })
    }
}

impl<T> Message for HashSet<T>
where
    T: Message + Eq + Hash + Serialize + DeserializeOwned,
{
    type Codec = SerdeCdrCodec<Self>;

    fn type_name() -> String {
        format!("HashSet<{}>", T::type_name())
    }
}

impl<T> MessageSchema for HashSet<T>
where
    T: MessageSchema,
{
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        Ok(TypeDef::Sequence {
            element: Box::new(T::build_schema(builder)?),
            length: SequenceLengthDef::Dynamic,
        })
    }
}

impl<K, V> Message for HashMap<K, V>
where
    K: Message + Eq + Hash + Serialize + DeserializeOwned,
    V: Message + Serialize + DeserializeOwned,
{
    type Codec = SerdeCdrCodec<Self>;

    fn type_name() -> String {
        format!("HashMap<{},{}>", K::type_name(), V::type_name())
    }
}

impl<K, V> MessageSchema for HashMap<K, V>
where
    K: MessageSchema,
    V: MessageSchema,
{
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        Ok(TypeDef::Map {
            key: Box::new(K::build_schema(builder)?),
            value: Box::new(V::build_schema(builder)?),
        })
    }
}

impl<K, V> Message for BTreeMap<K, V>
where
    K: Message + Ord + Serialize + DeserializeOwned,
    V: Message + Serialize + DeserializeOwned,
{
    type Codec = SerdeCdrCodec<Self>;

    fn type_name() -> String {
        format!("BTreeMap<{},{}>", K::type_name(), V::type_name())
    }
}

impl<K, V> MessageSchema for BTreeMap<K, V>
where
    K: MessageSchema,
    V: MessageSchema,
{
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        Ok(TypeDef::Map {
            key: Box::new(K::build_schema(builder)?),
            value: Box::new(V::build_schema(builder)?),
        })
    }
}

/// Serde-backed CDR codec for message types deriving `Serialize` and `Deserialize`.
pub struct SerdeCdrCodec<T>(PhantomData<T>);

/// Codec-side decoder used by subscribers and service handlers.
pub trait WireDecoder {
    /// Input accepted by the decoder, usually bytes or a Zenoh buffer.
    type Input<'a>;
    /// Decoded output type.
    type Output;
    /// Decode error type.
    type Error: std::error::Error + Send + Sync + 'static;
    /// Decode one value from `input`.
    fn deserialize(input: Self::Input<'_>) -> Result<Self::Output, Self::Error>;
}

// ── Serde-backed CDR serialization for typed messages ─────────────────────────────────────

/// CDR encapsulation header for little-endian encoding
pub const CDR_HEADER_LE: [u8; 4] = [0x00, 0x01, 0x00, 0x00];

impl<T> WireEncoder for SerdeCdrCodec<T>
where
    T: Serialize,
{
    type Input<'a>
        = &'a T
    where
        T: 'a;

    fn serialize_to_zbuf(input: &T) -> ZBuf {
        Self::serialize_to_zbuf_with_hint(input, 256)
    }

    fn serialize_to_zbuf_with_hint(input: &T, capacity_hint: usize) -> ZBuf {
        let mut writer = ZBufWriter::with_capacity(capacity_hint);
        writer.extend_from_slice(&CDR_HEADER_LE);
        let mut serializer = SerdeCdrSerializer::<LittleEndian, ZBufWriter>::new(&mut writer);
        input.serialize(&mut serializer).unwrap();
        writer.into_zbuf()
    }

    fn serialized_size_hint(_input: &T) -> usize {
        std::mem::size_of::<T>() * 2 + 4
    }

    fn serialize_to_shm(
        input: &T,
        estimated_size: usize,
        provider: &ShmProvider<PosixShmProviderBackend>,
    ) -> zenoh::Result<(ZBuf, usize)> {
        let mut writer = ShmWriter::new(provider, estimated_size)?;
        writer.extend_from_slice(&CDR_HEADER_LE);
        let mut serializer = SerdeCdrSerializer::<LittleEndian, ShmWriter>::new(&mut writer);
        input
            .serialize(&mut serializer)
            .map_err(|e| zenoh::Error::from(format!("CDR serialization failed: {}", e)))?;
        let actual_size = writer.position();
        let zbuf = writer.into_zbuf()?;
        Ok((zbuf, actual_size))
    }

    fn serialize(input: &T) -> Vec<u8> {
        let mut buffer = Vec::new();
        Self::serialize_to_buf(input, &mut buffer);
        buffer
    }

    fn serialize_to_buf(input: &T, buffer: &mut Vec<u8>) {
        buffer.clear();
        buffer.extend_from_slice(&CDR_HEADER_LE);
        let mut fast_ser = SerdeCdrSerializer::<LittleEndian>::new(buffer);
        input.serialize(&mut fast_ser).unwrap();
    }
}

impl<T> WireDecoder for SerdeCdrCodec<T>
where
    T: DeserializeOwned,
{
    type Input<'a> = &'a [u8];
    type Output = T;
    type Error = CdrError;

    fn deserialize(input: Self::Input<'_>) -> Result<Self::Output, Self::Error> {
        if input.len() < 4 {
            return Err(CdrError("CDR data too short for header".into()));
        }
        let representation_identifier = &input[0..2];
        if representation_identifier != [0x00, 0x01] {
            return Err(CdrError(format!(
                "Expected CDR_LE encapsulation ({:?}), found {:?}",
                [0x00, 0x01],
                representation_identifier
            )));
        }
        let payload = &input[4..];
        let x = ros_z_cdr::from_bytes::<T, byteorder::LittleEndian>(payload)
            .map_err(|e| CdrError(e.to_string()))?;
        Ok(x.0)
    }
}

/// Service contract pairing request and response wire message types.
pub trait Service {
    /// Request message accepted by the service server.
    type Request: Message;
    /// Response message returned by the service server.
    type Response: Message;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use zenoh_buffers::buffer::SplitBuffer;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct SimpleMessage {
        value: u32,
        text: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct LargeMessage {
        data: Vec<u8>,
        count: u64,
        nested: Vec<SimpleMessage>,
    }

    #[test]
    fn test_serialize_to_zbuf() {
        let message = SimpleMessage {
            value: 42,
            text: "Hello, ZBuf!".to_string(),
        };

        let zbuf = SerdeCdrCodec::<SimpleMessage>::serialize_to_zbuf(&message);
        let bytes = zbuf.contiguous();

        // Verify CDR header
        assert_eq!(&bytes[0..4], &CDR_HEADER_LE);

        // Verify roundtrip
        let deserialized = SerdeCdrCodec::<SimpleMessage>::deserialize(&bytes).unwrap();
        assert_eq!(deserialized, message);
    }

    #[test]
    fn test_serialize_to_zbuf_consistency() {
        let message = SimpleMessage {
            value: 123,
            text: "consistency test".to_string(),
        };

        // Both methods should produce identical bytes
        let zbuf = SerdeCdrCodec::<SimpleMessage>::serialize_to_zbuf(&message);
        let vec = SerdeCdrCodec::<SimpleMessage>::serialize(&message);

        let zbuf_bytes = zbuf.contiguous();
        assert_eq!(&*zbuf_bytes, &vec[..]);
    }

    #[test]
    fn test_serialize_to_zbuf_reuse() {
        let msg1 = SimpleMessage {
            value: 1,
            text: "first".to_string(),
        };
        let msg2 = SimpleMessage {
            value: 2,
            text: "second".to_string(),
        };

        let mut buffer = Vec::with_capacity(1024);

        // First serialization
        let zbuf1 = SerdeCdrCodec::<SimpleMessage>::serialize_to_zbuf_reuse(&msg1, &mut buffer);
        let bytes1 = zbuf1.contiguous();

        // Buffer should be empty after take
        assert!(buffer.is_empty());

        // Second serialization (buffer will be reallocated)
        let zbuf2 = SerdeCdrCodec::<SimpleMessage>::serialize_to_zbuf_reuse(&msg2, &mut buffer);
        let bytes2 = zbuf2.contiguous();

        // Verify roundtrips
        let decoded1 = SerdeCdrCodec::<SimpleMessage>::deserialize(&bytes1).unwrap();
        let decoded2 = SerdeCdrCodec::<SimpleMessage>::deserialize(&bytes2).unwrap();

        assert_eq!(decoded1, msg1);
        assert_eq!(decoded2, msg2);
    }

    #[test]
    fn test_zmessage_serialize_to_zbuf() {
        let message = SimpleMessage {
            value: 777,
            text: "trait test".to_string(),
        };

        // Codec provides serialize_to_zbuf with a size hint.
        let zbuf = SerdeCdrCodec::<SimpleMessage>::serialize_to_zbuf_with_hint(
            &message,
            SerdeCdrCodec::<SimpleMessage>::serialized_size_hint(&message),
        );
        let bytes = zbuf.contiguous();

        assert_eq!(&bytes[0..4], &CDR_HEADER_LE);

        let deserialized = SerdeCdrCodec::<SimpleMessage>::deserialize(&bytes).unwrap();
        assert_eq!(deserialized, message);
    }

    #[test]
    fn test_cdr_encode_to_buf_consistency() {
        let message = SimpleMessage {
            value: 42,
            text: "Hello, ros-z!".to_string(),
        };

        // Serialize using both methods
        let vec1 = SerdeCdrCodec::<SimpleMessage>::serialize(&message);
        let mut vec2 = Vec::new();
        SerdeCdrCodec::<SimpleMessage>::serialize_to_buf(&message, &mut vec2);

        // Results should be identical
        assert_eq!(vec1, vec2);
        assert!(!vec1.is_empty());
        assert_eq!(&vec1[0..4], &CDR_HEADER_LE); // CDR header
    }

    #[test]
    fn serialize_to_buf_replaces_previous_larger_payload() {
        let msg1 = LargeMessage {
            data: vec![1; 1000],
            count: 100,
            nested: vec![],
        };

        let msg2 = SimpleMessage {
            value: 1,
            text: "x".to_string(),
        };

        let mut buffer = Vec::new();

        // Serialize large message
        SerdeCdrCodec::<LargeMessage>::serialize_to_buf(&msg1, &mut buffer);
        let len1 = buffer.len();
        assert!(len1 > 100);

        // Serialize small message - should clear buffer first
        SerdeCdrCodec::<SimpleMessage>::serialize_to_buf(&msg2, &mut buffer);
        let len2 = buffer.len();
        assert!(len2 < len1);

        // Verify content is correct (not mixed)
        assert_eq!(&buffer[0..4], &CDR_HEADER_LE); // CDR header
    }

    #[test]
    fn test_cdr_roundtrip_with_serialize_to_buf() {
        let original = LargeMessage {
            data: vec![1, 2, 3, 4, 5, 6, 7, 8],
            count: 42,
            nested: vec![
                SimpleMessage {
                    value: 10,
                    text: "first".to_string(),
                },
                SimpleMessage {
                    value: 20,
                    text: "second".to_string(),
                },
            ],
        };

        // Serialize using serialize_to_buf
        let mut buffer = Vec::new();
        SerdeCdrCodec::<LargeMessage>::serialize_to_buf(&original, &mut buffer);

        // Deserialize
        let deserialized =
            SerdeCdrCodec::<LargeMessage>::deserialize(&buffer).expect("Failed to deserialize");

        // Should match original
        assert_eq!(deserialized, original);
    }

    #[test]
    fn serialize_to_buf_can_be_reused_for_multiple_distinct_messages() {
        let messages = vec![
            SimpleMessage {
                value: 1,
                text: "one".to_string(),
            },
            SimpleMessage {
                value: 2,
                text: "two".to_string(),
            },
            SimpleMessage {
                value: 3,
                text: "three".to_string(),
            },
        ];

        let mut buffer = Vec::new();
        let mut all_serialized = Vec::new();

        for message in &messages {
            SerdeCdrCodec::<SimpleMessage>::serialize_to_buf(message, &mut buffer);
            all_serialized.push(buffer.clone());

            // Verify each serialization is correct
            let deserialized = SerdeCdrCodec::<SimpleMessage>::deserialize(&buffer)
                .expect("Failed to deserialize");
            assert_eq!(&deserialized, message);
        }

        // Verify all serializations are different
        assert_ne!(all_serialized[0], all_serialized[1]);
        assert_ne!(all_serialized[1], all_serialized[2]);
    }

    #[test]
    fn test_zmessage_trait_implementation() {
        let message = SimpleMessage {
            value: 777,
            text: "trait test".to_string(),
        };

        // Codec provides serialize method.
        let serialized = SerdeCdrCodec::<SimpleMessage>::serialize(&message);
        assert!(!serialized.is_empty());
        assert_eq!(&serialized[0..4], &CDR_HEADER_LE);

        // Codec provides deserialize method.
        let deserialized = SerdeCdrCodec::<SimpleMessage>::deserialize(&serialized[..])
            .expect("Failed to deserialize");
        assert_eq!(deserialized, message);
    }
}
