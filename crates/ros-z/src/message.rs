use byteorder::LittleEndian;
use ros_z_cdr::{CdrBuffer, CdrReader, CdrWriter};
use ros_z_schema::{
    PrimitiveTypeDef, SchemaBundle, SchemaError, SchemaHash, SequenceLengthDef, TypeDef, TypeName,
};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::Hash;
use std::net::SocketAddr;
use std::ops::{Range, RangeInclusive};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use zenoh::shm::{PosixShmProviderBackend, ShmProvider};
use zenoh_buffers::ZBuf;

pub use crate::CdrCodec;
use crate::attachment::{ENDPOINT_GLOBAL_ID_SIZE, EndpointGlobalId};
use crate::entity::TypeInfo;
use crate::schema::{MessageSchema, SchemaBuilder};

pub mod report;

/// Error returned when CDR bytes cannot be decoded into the requested type.
#[derive(Debug, thiserror::Error)]
pub enum CdrError {
    #[error("CDR deserialization error: {0}")]
    Message(String),

    #[error("CDR deserialization error: {0}")]
    Cdr(#[from] ros_z_cdr::Error),
}

/// Error returned when CDR bytes cannot be encoded from the requested type.
#[derive(Debug, thiserror::Error)]
pub enum CdrEncodeError {
    #[error(transparent)]
    Cdr(#[from] ros_z_cdr::Error),
}

/// Codec-side encoder used by publishers and prepared publications.
pub trait WireEncoder {
    type Input<'a>
    where
        Self: 'a;

    type Error: std::error::Error + Send + Sync + 'static;

    /// Serialize directly to a ZBuf for zero-copy publishing.
    ///
    /// This is the primary serialization method that returns a ZBuf,
    /// optimized for Zenoh publishing without intermediate copies.
    ///
    /// Uses a fixed 256-byte initial capacity. For better performance with
    /// large messages, use `serialize_to_zbuf_with_hint()` when the caller has
    /// a better capacity estimate.
    fn serialize_to_zbuf(input: Self::Input<'_>) -> Result<ZBuf, Self::Error>;

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
    /// use ros_z::message::{WireEncoder, CdrCodec};
    /// use serde::Serialize;
    ///
    /// #[derive(Serialize, serde::Deserialize, ros_z::Message)]
    /// struct LargeMsg { data: Vec<u8> }
    ///
    /// let message = LargeMsg { data: vec![0; 1_000_000] };
    /// let hint = 4 + 4 + 1_000_000;  // header + length + data
    /// let zbuf = CdrCodec::<LargeMsg>::serialize_to_zbuf_with_hint(&message, hint)?;
    /// # Ok::<(), ros_z::message::CdrEncodeError>(())
    /// ```
    fn serialize_to_zbuf_with_hint(
        input: Self::Input<'_>,
        capacity_hint: usize,
    ) -> Result<ZBuf, Self::Error>;

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
    /// Returns an error if encoding, SHM allocation, or ZBuf conversion fails.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ros_z::message::{WireEncoder, CdrCodec};
    /// use ros_z::shm::ShmProviderBuilder;
    /// use serde::Serialize;
    ///
    /// # fn main() -> ros_z::Result<()> {
    /// #[derive(Serialize, serde::Deserialize, ros_z::Message)]
    /// struct MyMsg { value: u32 }
    ///
    /// let message = MyMsg { value: 42 };
    /// let provider = ShmProviderBuilder::new(1024 * 1024).build()?;
    ///
    /// let (zbuf, size) = CdrCodec::<MyMsg>::serialize_to_shm(&message, 128, &provider)?;
    /// println!("Serialized {} bytes to SHM", size);
    /// # Ok(())
    /// # }
    /// ```
    fn serialize_to_shm(
        input: Self::Input<'_>,
        estimated_size: usize,
        provider: &ShmProvider<PosixShmProviderBackend>,
    ) -> crate::Result<(ZBuf, usize)>;

    /// Serialize to an existing buffer, returning the result as ZBuf.
    ///
    /// This variant allows buffer reuse for reduced allocations.
    /// The buffer is cleared and reused, then wrapped in a ZBuf.
    fn serialize_to_zbuf_reuse(
        input: Self::Input<'_>,
        buffer: &mut Vec<u8>,
    ) -> Result<ZBuf, Self::Error> {
        Self::serialize_to_buf(input, buffer)?;
        // Take ownership of the buffer contents, leaving an empty Vec
        Ok(ZBuf::from(std::mem::take(buffer)))
    }

    /// Serialize to an owned byte vector for callers that need contiguous bytes.
    ///
    /// Prefer `serialize_to_zbuf()` for zero-copy publishing.
    fn serialize(input: Self::Input<'_>) -> Result<Vec<u8>, Self::Error> {
        let mut buffer = Vec::new();
        Self::serialize_to_buf(input, &mut buffer)?;
        Ok(buffer)
    }

    /// Serialize to an existing buffer, reusing its allocation.
    ///
    /// The buffer is cleared before writing. Implementations should
    /// write directly to the buffer for optimal performance.
    fn serialize_to_buf(input: Self::Input<'_>, buffer: &mut Vec<u8>) -> Result<(), Self::Error>;
}

/// Typed message contract for ros-z publishers, subscribers, services, and schemas.
pub trait Message: MessageSchema + Send + Sync + 'static {
    /// Field-level CDR encoding, without an encapsulation header.
    fn encode_cdr<B: CdrBuffer>(
        &self,
        writer: &mut CdrWriter<'_, LittleEndian, B>,
    ) -> ros_z_cdr::Result<()>;

    /// Field-level decoding; the reader retains any owned payload segments.
    fn decode_cdr(reader: &mut CdrReader<'_, LittleEndian>) -> ros_z_cdr::Result<Self>
    where
        Self: Sized;
    /// Encode consecutive elements, without a sequence length prefix.
    fn encode_cdr_slice<B: CdrBuffer>(
        values: &[Self],
        writer: &mut CdrWriter<'_, LittleEndian, B>,
    ) -> ros_z_cdr::Result<()>
    where
        Self: Sized,
    {
        for value in values {
            value.encode_cdr(writer)?;
        }
        Ok(())
    }

    /// Decode consecutive elements after the caller has read the length.
    fn decode_cdr_vec(
        reader: &mut CdrReader<'_, LittleEndian>,
        count: usize,
    ) -> ros_z_cdr::Result<Vec<Self>>
    where
        Self: Sized,
    {
        let mut values =
            Vec::with_capacity(count.min(1024 * 1024 / std::mem::size_of::<Self>().max(1)));
        for _ in 0..count {
            values.push(Self::decode_cdr(reader)?);
        }
        Ok(values)
    }

    /// Decode a fixed array without a sequence length prefix.
    fn decode_cdr_array<const N: usize>(
        reader: &mut CdrReader<'_, LittleEndian>,
    ) -> ros_z_cdr::Result<[Self; N]>
    where
        Self: Sized,
    {
        let mut values = std::array::from_fn::<_, N, _>(|_| None);
        for value in &mut values {
            *value = Some(Self::decode_cdr(reader)?);
        }
        Ok(values.map(|value| value.expect("all array elements decoded")))
    }
    /// Codec used to encode and decode this message type.
    type Codec: for<'a> WireEncoder<Input<'a> = &'a Self>
        + for<'a> WireDecoder<Input<'a> = &'a [u8], Output = Self>;

    /// Stable fully qualified type name advertised in graph metadata.
    fn type_name() -> String;
    /// Runtime schema used for discovery and dynamic tooling.
    ///
    /// # Panics
    ///
    /// Panics if this message type builds an invalid static schema.
    fn schema() -> SchemaBundle {
        crate::schema::schema_for::<Self>().expect("message schema should be static and valid")
    }

    /// Stable hash derived from [`Message::schema`].
    ///
    /// # Panics
    ///
    /// Panics if this message type builds a schema that cannot be hashed.
    fn schema_hash() -> SchemaHash {
        ros_z_schema::compute_hash(&Self::schema())
            .expect("message schema hash should be static and valid")
    }

    /// Type name plus schema hash advertised for this message.
    ///
    /// # Panics
    ///
    /// Panics if [`Message::schema_hash`] panics.
    fn type_info() -> TypeInfo {
        TypeInfo::new(Self::type_name(), Self::schema_hash())
    }
}

pub(crate) fn validated_type_info_for_schema<T>(schema: &SchemaBundle) -> TypeInfo
where
    T: Message,
{
    let type_info = T::type_info();
    let type_name = T::type_name();
    if let TypeDef::Named(root_name) = &schema.root
        && root_name.as_str() != type_name
    {
        panic!(
            "message schema root should match static type name: schema root '{}' does not match Message::type_name() '{}'",
            root_name.as_str(),
            type_name
        );
    }

    if type_info.name != type_name {
        panic!(
            "message type info should match static schema metadata: type_info name '{}' does not match Message::type_name() '{}'",
            type_info.name, type_name
        );
    }

    let schema_hash =
        ros_z_schema::compute_hash(schema).expect("message schema hash should be static and valid");
    if type_info.hash != schema_hash {
        panic!(
            "message type info should match static schema metadata: type_info hash '{}' does not match computed schema hash '{}'",
            type_info.hash.to_hash_string(),
            schema_hash.to_hash_string()
        );
    }

    type_info
}

macro_rules! impl_bulk_cdr {
    () => {};
    (bulk) => {
        #[cfg(target_endian = "little")]
        fn encode_cdr_slice<B: CdrBuffer>(
            values: &[Self],
            writer: &mut CdrWriter<'_, LittleEndian, B>,
        ) -> ros_z_cdr::Result<()> {
            if !values.is_empty() {
                writer.write_pod_slice(values);
            }
            Ok(())
        }
        #[cfg(target_endian = "little")]
        fn decode_cdr_vec(
            reader: &mut CdrReader<'_, LittleEndian>,
            count: usize,
        ) -> ros_z_cdr::Result<Vec<Self>> {
            reader.read_pod_slice(count)
        }
        #[cfg(target_endian = "little")]
        fn decode_cdr_array<const N: usize>(
            reader: &mut CdrReader<'_, LittleEndian>,
        ) -> ros_z_cdr::Result<[Self; N]> {
            reader.read_pod_array()
        }
    };
}

macro_rules! impl_primitive_message {
    ($ty:ty, $name:literal, $primitive:ident $(, $bulk:ident)?) => {
        impl MessageSchema for $ty {
            fn build_schema(_builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
                Ok(TypeDef::Primitive(PrimitiveTypeDef::$primitive))
            }
        }

        impl Message for $ty {
            impl_bulk_cdr!($($bulk)?);
            type Codec = CdrCodec<Self>;
            fn encode_cdr<B: CdrBuffer>(
                &self,
                writer: &mut CdrWriter<'_, LittleEndian, B>,
            ) -> ros_z_cdr::Result<()> {
                crate::serde_bridge::encode_serde(self, writer)
            }

            fn decode_cdr(reader: &mut CdrReader<'_, LittleEndian>) -> ros_z_cdr::Result<Self> {
                crate::serde_bridge::decode_serde(reader)
            }

            fn type_name() -> String {
                $name.to_string()
            }
        }
    };
}

impl_primitive_message!(bool, "bool", Bool);
impl_primitive_message!(i8, "i8", I8, bulk);
impl_primitive_message!(u8, "u8", U8, bulk);
impl_primitive_message!(i16, "i16", I16, bulk);
impl_primitive_message!(u16, "u16", U16, bulk);
impl_primitive_message!(i32, "i32", I32, bulk);
impl_primitive_message!(u32, "u32", U32, bulk);
impl_primitive_message!(i64, "i64", I64, bulk);
impl_primitive_message!(u64, "u64", U64, bulk);
impl_primitive_message!(usize, "usize", U64);
impl_primitive_message!(f32, "f32", F32, bulk);
impl_primitive_message!(f64, "f64", F64, bulk);
impl_primitive_message!((), "()", Unit);

impl<T> Message for Box<T>
where
    T: Message,
{
    type Codec = crate::CdrCodec<Self>;
    fn encode_cdr<B: CdrBuffer>(
        &self,
        w: &mut CdrWriter<'_, LittleEndian, B>,
    ) -> ros_z_cdr::Result<()> {
        (**self).encode_cdr(w)
    }
    fn decode_cdr(r: &mut CdrReader<'_, LittleEndian>) -> ros_z_cdr::Result<Self> {
        Ok(Box::new(T::decode_cdr(r)?))
    }

    fn type_name() -> String {
        format!("Box<{}>", T::type_name())
    }
}

impl<T> MessageSchema for Box<T>
where
    T: MessageSchema + ?Sized,
{
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        T::build_schema(builder)
    }
}

impl<T> Message for Arc<T>
where
    T: Message,
{
    type Codec = crate::CdrCodec<Self>;
    fn encode_cdr<B: CdrBuffer>(
        &self,
        w: &mut CdrWriter<'_, LittleEndian, B>,
    ) -> ros_z_cdr::Result<()> {
        (**self).encode_cdr(w)
    }
    fn decode_cdr(r: &mut CdrReader<'_, LittleEndian>) -> ros_z_cdr::Result<Self> {
        Ok(Arc::new(T::decode_cdr(r)?))
    }

    fn type_name() -> String {
        format!("Arc<{}>", T::type_name())
    }
}

impl<T> MessageSchema for Arc<T>
where
    T: MessageSchema + ?Sized,
{
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        T::build_schema(builder)
    }
}

impl<T: Message> Message for Arc<[T]> {
    type Codec = crate::CdrCodec<Self>;
    fn type_name() -> String {
        format!("Arc<[{}]>", T::type_name())
    }
    fn encode_cdr<B: CdrBuffer>(
        &self,
        w: &mut CdrWriter<'_, LittleEndian, B>,
    ) -> ros_z_cdr::Result<()> {
        crate::native_cdr::write_len(w, self.len())?;
        T::encode_cdr_slice(self, w)
    }
    fn decode_cdr(r: &mut CdrReader<'_, LittleEndian>) -> ros_z_cdr::Result<Self> {
        Ok(Vec::<T>::decode_cdr(r)?.into())
    }
}

impl<T: Message> Message for Box<[T]> {
    type Codec = crate::CdrCodec<Self>;
    fn type_name() -> String {
        format!("Box<[{}]>", T::type_name())
    }
    fn encode_cdr<B: CdrBuffer>(
        &self,
        w: &mut CdrWriter<'_, LittleEndian, B>,
    ) -> ros_z_cdr::Result<()> {
        crate::native_cdr::write_len(w, self.len())?;
        T::encode_cdr_slice(self, w)
    }
    fn decode_cdr(r: &mut CdrReader<'_, LittleEndian>) -> ros_z_cdr::Result<Self> {
        Ok(Vec::<T>::decode_cdr(r)?.into())
    }
}

impl Message for Duration {
    type Codec = CdrCodec<Self>;
    fn encode_cdr<B: CdrBuffer>(
        &self,
        writer: &mut CdrWriter<'_, LittleEndian, B>,
    ) -> ros_z_cdr::Result<()> {
        crate::serde_bridge::encode_serde(self, writer)
    }

    fn decode_cdr(reader: &mut CdrReader<'_, LittleEndian>) -> ros_z_cdr::Result<Self> {
        crate::serde_bridge::decode_serde(reader)
    }

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
    type Codec = CdrCodec<Self>;
    fn encode_cdr<B: CdrBuffer>(
        &self,
        writer: &mut CdrWriter<'_, LittleEndian, B>,
    ) -> ros_z_cdr::Result<()> {
        crate::serde_bridge::encode_serde(self, writer)
    }

    fn decode_cdr(reader: &mut CdrReader<'_, LittleEndian>) -> ros_z_cdr::Result<Self> {
        crate::serde_bridge::decode_serde(reader)
    }

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
    T: Message,
{
    type Codec = crate::CdrCodec<Self>;
    fn encode_cdr<B: CdrBuffer>(
        &self,
        w: &mut CdrWriter<'_, LittleEndian, B>,
    ) -> ros_z_cdr::Result<()> {
        self.start.encode_cdr(w)?;
        self.end.encode_cdr(w)
    }
    fn decode_cdr(r: &mut CdrReader<'_, LittleEndian>) -> ros_z_cdr::Result<Self> {
        Ok(T::decode_cdr(r)?..T::decode_cdr(r)?)
    }

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
    T: Message,
{
    type Codec = crate::CdrCodec<Self>;
    fn encode_cdr<B: CdrBuffer>(
        &self,
        w: &mut CdrWriter<'_, LittleEndian, B>,
    ) -> ros_z_cdr::Result<()> {
        self.start().encode_cdr(w)?;
        self.end().encode_cdr(w)
    }
    fn decode_cdr(r: &mut CdrReader<'_, LittleEndian>) -> ros_z_cdr::Result<Self> {
        Ok(T::decode_cdr(r)?..=T::decode_cdr(r)?)
    }

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
    type Codec = CdrCodec<Self>;
    fn encode_cdr<B: CdrBuffer>(
        &self,
        writer: &mut CdrWriter<'_, LittleEndian, B>,
    ) -> ros_z_cdr::Result<()> {
        crate::serde_bridge::encode_serde(self, writer)
    }

    fn decode_cdr(reader: &mut CdrReader<'_, LittleEndian>) -> ros_z_cdr::Result<Self> {
        crate::serde_bridge::decode_serde(reader)
    }

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
    type Codec = CdrCodec<Self>;
    fn encode_cdr<B: CdrBuffer>(
        &self,
        writer: &mut CdrWriter<'_, LittleEndian, B>,
    ) -> ros_z_cdr::Result<()> {
        crate::serde_bridge::encode_serde(self, writer)
    }

    fn decode_cdr(reader: &mut CdrReader<'_, LittleEndian>) -> ros_z_cdr::Result<Self> {
        crate::serde_bridge::decode_serde(reader)
    }

    fn type_name() -> String {
        "String".to_string()
    }
}

impl MessageSchema for String {
    fn build_schema(_builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        Ok(TypeDef::String)
    }
}

impl<T, E> Message for Result<T, E>
where
    T: Message,
    E: Message,
{
    type Codec = CdrCodec<Self>;

    fn encode_cdr<B: CdrBuffer>(
        &self,
        writer: &mut CdrWriter<'_, LittleEndian, B>,
    ) -> ros_z_cdr::Result<()> {
        match self {
            Ok(value) => {
                writer.write_u32(0);
                value.encode_cdr(writer)
            }
            Err(error) => {
                writer.write_u32(1);
                error.encode_cdr(writer)
            }
        }
    }

    fn decode_cdr(reader: &mut CdrReader<'_, LittleEndian>) -> ros_z_cdr::Result<Self> {
        match reader.read_u32()? {
            0 => Ok(Ok(T::decode_cdr(reader)?)),
            1 => Ok(Err(E::decode_cdr(reader)?)),
            tag => Err(ros_z_cdr::Error::Custom(format!(
                "invalid Result tag {tag}"
            ))),
        }
    }

    fn type_name() -> String {
        format!("Result<{},{}>", T::type_name(), E::type_name())
    }
}

impl<T, E> MessageSchema for Result<T, E>
where
    T: Message,
    E: Message,
{
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        let name = TypeName::new(format!("Result<{},{}>", T::type_name(), E::type_name()))?;
        builder.define_enum(name, |variants| {
            variants.newtype::<T>("Ok")?;
            variants.newtype::<E>("Err")?;
            Ok(())
        })
    }
}

impl<T> Message for Option<T>
where
    T: Message,
{
    type Codec = crate::CdrCodec<Self>;
    fn encode_cdr<B: CdrBuffer>(
        &self,
        w: &mut CdrWriter<'_, LittleEndian, B>,
    ) -> ros_z_cdr::Result<()> {
        w.write_u32(u32::from(self.is_some()));
        if let Some(value) = self {
            value.encode_cdr(w)?;
        }
        Ok(())
    }
    fn decode_cdr(r: &mut CdrReader<'_, LittleEndian>) -> ros_z_cdr::Result<Self> {
        match r.read_u32()? {
            0 => Ok(None),
            1 => Ok(Some(T::decode_cdr(r)?)),
            tag => Err(ros_z_cdr::Error::InvalidOptionTag(tag)),
        }
    }

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
    T: Message,
{
    type Codec = crate::CdrCodec<Self>;
    fn encode_cdr<B: CdrBuffer>(
        &self,
        w: &mut CdrWriter<'_, LittleEndian, B>,
    ) -> ros_z_cdr::Result<()> {
        crate::native_cdr::write_len(w, self.len())?;
        T::encode_cdr_slice(self, w)
    }
    fn decode_cdr(r: &mut CdrReader<'_, LittleEndian>) -> ros_z_cdr::Result<Self> {
        let count = r.read_sequence_length()?;
        T::decode_cdr_vec(r, count)
    }

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
    T: Message,
{
    type Codec = crate::CdrCodec<Self>;
    fn encode_cdr<B: CdrBuffer>(
        &self,
        w: &mut CdrWriter<'_, LittleEndian, B>,
    ) -> ros_z_cdr::Result<()> {
        T::encode_cdr_slice(self, w)
    }
    fn decode_cdr(r: &mut CdrReader<'_, LittleEndian>) -> ros_z_cdr::Result<Self> {
        T::decode_cdr_array(r)
    }

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

impl<T> MessageSchema for [T]
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

impl MessageSchema for EndpointGlobalId {
    fn build_schema(builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        <[u8; ENDPOINT_GLOBAL_ID_SIZE]>::build_schema(builder)
    }
}

impl Message for EndpointGlobalId {
    type Codec = CdrCodec<Self>;
    fn encode_cdr<B: CdrBuffer>(
        &self,
        writer: &mut CdrWriter<'_, LittleEndian, B>,
    ) -> ros_z_cdr::Result<()> {
        crate::serde_bridge::encode_serde(self, writer)
    }

    fn decode_cdr(reader: &mut CdrReader<'_, LittleEndian>) -> ros_z_cdr::Result<Self> {
        crate::serde_bridge::decode_serde(reader)
    }
    fn type_name() -> String {
        "ros_z_protocol::EndpointGlobalId".into()
    }
}

impl<T> Message for HashSet<T>
where
    T: Message + Eq + Hash,
{
    type Codec = crate::CdrCodec<Self>;
    fn encode_cdr<B: CdrBuffer>(
        &self,
        w: &mut CdrWriter<'_, LittleEndian, B>,
    ) -> ros_z_cdr::Result<()> {
        crate::native_cdr::write_len(w, self.len())?;
        for value in self {
            value.encode_cdr(w)?;
        }
        Ok(())
    }
    fn decode_cdr(r: &mut CdrReader<'_, LittleEndian>) -> ros_z_cdr::Result<Self> {
        let count = r.read_sequence_length()?;
        let mut values = HashSet::new();
        for _ in 0..count {
            values.insert(T::decode_cdr(r)?);
        }
        Ok(values)
    }

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
    K: Message + Eq + Hash,
    V: Message,
{
    type Codec = crate::CdrCodec<Self>;
    fn encode_cdr<B: CdrBuffer>(
        &self,
        w: &mut CdrWriter<'_, LittleEndian, B>,
    ) -> ros_z_cdr::Result<()> {
        crate::native_cdr::write_len(w, self.len())?;
        for (key, value) in self {
            key.encode_cdr(w)?;
            value.encode_cdr(w)?;
        }
        Ok(())
    }
    fn decode_cdr(r: &mut CdrReader<'_, LittleEndian>) -> ros_z_cdr::Result<Self> {
        let count = r.read_sequence_length()?;
        let mut values = HashMap::new();
        for _ in 0..count {
            values.insert(K::decode_cdr(r)?, V::decode_cdr(r)?);
        }
        Ok(values)
    }

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
    K: Message + Ord,
    V: Message,
{
    type Codec = crate::CdrCodec<Self>;
    fn encode_cdr<B: CdrBuffer>(
        &self,
        w: &mut CdrWriter<'_, LittleEndian, B>,
    ) -> ros_z_cdr::Result<()> {
        crate::native_cdr::write_len(w, self.len())?;
        for (key, value) in self {
            key.encode_cdr(w)?;
            value.encode_cdr(w)?;
        }
        Ok(())
    }
    fn decode_cdr(r: &mut CdrReader<'_, LittleEndian>) -> ros_z_cdr::Result<Self> {
        let count = r.read_sequence_length()?;
        let mut values = BTreeMap::new();
        for _ in 0..count {
            values.insert(K::decode_cdr(r)?, V::decode_cdr(r)?);
        }
        Ok(values)
    }

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

impl Message for PathBuf {
    type Codec = CdrCodec<Self>;
    fn encode_cdr<B: CdrBuffer>(
        &self,
        writer: &mut CdrWriter<'_, LittleEndian, B>,
    ) -> ros_z_cdr::Result<()> {
        crate::serde_bridge::encode_serde(self, writer)
    }

    fn decode_cdr(reader: &mut CdrReader<'_, LittleEndian>) -> ros_z_cdr::Result<Self> {
        crate::serde_bridge::decode_serde(reader)
    }

    fn type_name() -> String {
        "PathBuf".to_string()
    }
}

impl MessageSchema for PathBuf {
    fn build_schema(_builder: &mut SchemaBuilder) -> Result<TypeDef, SchemaError> {
        Ok(TypeDef::String)
    }
}

/// Codec-side decoder used by subscribers and service handlers.
pub trait WireDecoder {
    /// Owned-payload entry point; byte-input codecs retain their existing fallback.
    fn deserialize_payload(payload: &zenoh::bytes::ZBytes) -> Result<Self::Output, Self::Error>
    where
        for<'a> &'a [u8]: Into<Self::Input<'a>>,
    {
        Self::deserialize(payload.to_bytes().as_ref().into())
    }
    /// Input accepted by the decoder, usually bytes or a Zenoh buffer.
    type Input<'a>;
    /// Decoded output type.
    type Output;
    /// Decode error type.
    type Error: std::error::Error + Send + Sync + 'static;
    /// Decode one value from `input`.
    fn deserialize(input: Self::Input<'_>) -> Result<Self::Output, Self::Error>;
}

/// CDR encapsulation header for little-endian encoding
pub const CDR_HEADER_LE: [u8; 4] = [0x00, 0x01, 0x00, 0x00];

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
    use serde::{Deserialize, Serialize};
    use zenoh_buffers::buffer::SplitBuffer;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, crate::Message)]
    struct SimpleMessage {
        value: u32,
        text: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, crate::Message)]
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

        let zbuf = CdrCodec::<SimpleMessage>::serialize_to_zbuf(&message).unwrap();
        let bytes = zbuf.contiguous();

        // Verify CDR header
        assert_eq!(&bytes[0..4], &CDR_HEADER_LE);

        // Verify roundtrip
        let deserialized = CdrCodec::<SimpleMessage>::deserialize(&bytes).unwrap();
        assert_eq!(deserialized, message);
    }

    #[test]
    fn test_serialize_to_zbuf_consistency() {
        let message = SimpleMessage {
            value: 123,
            text: "consistency test".to_string(),
        };

        // Both methods should produce identical bytes
        let zbuf = CdrCodec::<SimpleMessage>::serialize_to_zbuf(&message).unwrap();
        let vec = CdrCodec::<SimpleMessage>::serialize(&message).unwrap();

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
        let zbuf1 = CdrCodec::<SimpleMessage>::serialize_to_zbuf_reuse(&msg1, &mut buffer).unwrap();
        let bytes1 = zbuf1.contiguous();

        // Buffer should be empty after take
        assert!(buffer.is_empty());

        // Second serialization (buffer will be reallocated)
        let zbuf2 = CdrCodec::<SimpleMessage>::serialize_to_zbuf_reuse(&msg2, &mut buffer).unwrap();
        let bytes2 = zbuf2.contiguous();

        // Verify roundtrips
        let decoded1 = CdrCodec::<SimpleMessage>::deserialize(&bytes1).unwrap();
        let decoded2 = CdrCodec::<SimpleMessage>::deserialize(&bytes2).unwrap();

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
        let zbuf = CdrCodec::<SimpleMessage>::serialize_to_zbuf_with_hint(
            &message,
            CdrCodec::<SimpleMessage>::serialized_size_hint(&message),
        )
        .unwrap();
        let bytes = zbuf.contiguous();

        assert_eq!(&bytes[0..4], &CDR_HEADER_LE);

        let deserialized = CdrCodec::<SimpleMessage>::deserialize(&bytes).unwrap();
        assert_eq!(deserialized, message);
    }

    #[test]
    fn test_cdr_encode_to_buf_consistency() {
        let message = SimpleMessage {
            value: 42,
            text: "Hello, ros-z!".to_string(),
        };

        // Serialize using both methods
        let vec1 = CdrCodec::<SimpleMessage>::serialize(&message).unwrap();
        let mut vec2 = Vec::new();
        CdrCodec::<SimpleMessage>::serialize_to_buf(&message, &mut vec2).unwrap();

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
        CdrCodec::<LargeMessage>::serialize_to_buf(&msg1, &mut buffer).unwrap();
        let len1 = buffer.len();
        assert!(len1 > 100);

        // Serialize small message - should clear buffer first
        CdrCodec::<SimpleMessage>::serialize_to_buf(&msg2, &mut buffer).unwrap();
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
        CdrCodec::<LargeMessage>::serialize_to_buf(&original, &mut buffer).unwrap();

        // Deserialize
        let deserialized =
            CdrCodec::<LargeMessage>::deserialize(&buffer).expect("Failed to deserialize");

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
            CdrCodec::<SimpleMessage>::serialize_to_buf(message, &mut buffer).unwrap();
            all_serialized.push(buffer.clone());

            // Verify each serialization is correct
            let deserialized =
                CdrCodec::<SimpleMessage>::deserialize(&buffer).expect("Failed to deserialize");
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
        let serialized = CdrCodec::<SimpleMessage>::serialize(&message).unwrap();
        assert!(!serialized.is_empty());
        assert_eq!(&serialized[0..4], &CDR_HEADER_LE);

        // Codec provides deserialize method.
        let deserialized =
            CdrCodec::<SimpleMessage>::deserialize(&serialized[..]).expect("Failed to deserialize");
        assert_eq!(deserialized, message);
    }
}
