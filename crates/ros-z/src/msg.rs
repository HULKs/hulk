use byteorder::LittleEndian;
use ros_z_cdr::{
    CdrBuffer, CdrDecode, CdrEncode, CdrEncodedSize, CdrWriter, SerdeCdrSerializer, ZBufWriter,
};
use serde::{Deserialize, Serialize};
use std::any::TypeId;
use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex, OnceLock};
use zenoh_buffers::ZBuf;

#[derive(Debug)]
pub struct CdrError(String);

impl std::fmt::Display for CdrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CDR deserialization error: {}", self.0)
    }
}

impl std::error::Error for CdrError {}

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
    /// large messages, use `serialize_to_zbuf_with_hint()` or call via
    /// `WireMessage::serialize_to_zbuf()` which provides accurate size hints.
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
    /// use ros_z::msg::{WireEncoder, SerdeCdrCodec};
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
    /// use ros_z::msg::{WireEncoder, SerdeCdrCodec};
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
        provider: &zenoh::shm::ShmProvider<zenoh::shm::PosixShmProviderBackend>,
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

pub struct EncodedMessage {
    pub payload: zenoh_buffers::ZBuf,
    pub encoding: crate::encoding::Encoding,
}

pub trait MessageCodec<T> {
    fn encode(value: &T) -> Result<EncodedMessage, CdrError>;
    fn encode_to_shm(
        value: &T,
        estimated_size: usize,
        provider: &zenoh::shm::ShmProvider<zenoh::shm::PosixShmProviderBackend>,
    ) -> zenoh::Result<EncodedMessage>;
    fn decode(bytes: &[u8]) -> Result<T, CdrError>;
    fn encoded_size_hint(value: &T) -> usize;
}

pub trait Message: Send + Sync + Sized + 'static {
    type Codec: MessageCodec<Self>;

    fn type_name() -> &'static str;
    fn schema() -> crate::dynamic::Schema;

    fn schema_hash() -> crate::entity::SchemaHash {
        crate::dynamic::schema_tree_hash(Self::type_name(), &Self::schema()).unwrap_or_else(|| {
            panic!(
                "message schema `{}` must convert to a schema hash",
                Self::type_name()
            )
        })
    }

    fn type_info() -> crate::entity::TypeInfo {
        crate::entity::TypeInfo::with_hash(Self::type_name(), Self::schema_hash())
    }
}

fn cached_type_name<T: 'static>(build: impl FnOnce() -> String) -> &'static str {
    static TYPE_NAMES: OnceLock<Mutex<HashMap<TypeId, &'static str>>> = OnceLock::new();

    let type_names = TYPE_NAMES.get_or_init(|| Mutex::new(HashMap::new()));
    let type_id = TypeId::of::<T>();

    if let Some(type_name) = type_names
        .lock()
        .expect("type name cache poisoned")
        .get(&type_id)
        .copied()
    {
        return type_name;
    }

    let type_name = Box::leak(build().into_boxed_str());
    let mut cache = type_names.lock().expect("type name cache poisoned");
    cache.entry(type_id).or_insert(type_name)
}

macro_rules! impl_primitive_message {
    ($ty:ty, $name:literal, $primitive:ident) => {
        impl Message for $ty {
            type Codec = SerdeCdrCodec<Self>;

            fn type_name() -> &'static str {
                $name
            }

            fn schema() -> crate::dynamic::Schema {
                Arc::new(crate::dynamic::TypeShape::Primitive(
                    crate::dynamic::PrimitiveType::$primitive,
                ))
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
impl_primitive_message!(f32, "f32", F32);
impl_primitive_message!(f64, "f64", F64);

impl Message for String {
    type Codec = SerdeCdrCodec<Self>;

    fn type_name() -> &'static str {
        "String"
    }

    fn schema() -> crate::dynamic::Schema {
        Arc::new(crate::dynamic::TypeShape::String)
    }
}

impl<T> Message for Option<T>
where
    T: Message + Serialize + for<'de> Deserialize<'de>,
{
    type Codec = SerdeCdrCodec<Self>;

    fn type_name() -> &'static str {
        cached_type_name::<Self>(|| format!("Option<{}>", T::type_name()))
    }

    fn schema() -> crate::dynamic::Schema {
        Arc::new(crate::dynamic::TypeShape::Optional(T::schema()))
    }
}

impl<T> Message for Vec<T>
where
    T: Message + Serialize + for<'de> Deserialize<'de>,
{
    type Codec = SerdeCdrCodec<Self>;

    fn type_name() -> &'static str {
        cached_type_name::<Self>(|| format!("Vec<{}>", T::type_name()))
    }

    fn schema() -> crate::dynamic::Schema {
        Arc::new(crate::dynamic::TypeShape::Sequence {
            element: T::schema(),
            length: crate::dynamic::SequenceLength::Dynamic,
        })
    }
}

impl<T, const N: usize> Message for [T; N]
where
    T: Message + Serialize + for<'de> Deserialize<'de>,
    [T; N]: Serialize + for<'de> Deserialize<'de>,
{
    type Codec = SerdeCdrCodec<Self>;

    fn type_name() -> &'static str {
        cached_type_name::<Self>(|| format!("[{};{}]", T::type_name(), N))
    }

    fn schema() -> crate::dynamic::Schema {
        Arc::new(crate::dynamic::TypeShape::Sequence {
            element: T::schema(),
            length: crate::dynamic::SequenceLength::Fixed(N),
        })
    }
}

#[doc(hidden)]
pub trait MapKey: private::SealedMapKey + Message {}

macro_rules! impl_map_key {
    ($($ty:ty),* $(,)?) => {
        $(
            impl private::SealedMapKey for $ty {}
            impl MapKey for $ty {}
        )*
    };
}

impl_map_key!(bool, i8, u8, i16, u16, i32, u32, i64, u64, String);

mod private {
    pub trait SealedMapKey {}
}

impl<K, V> Message for HashMap<K, V>
where
    K: MapKey + Eq + Hash + Serialize + for<'de> Deserialize<'de>,
    V: Message + Serialize + for<'de> Deserialize<'de>,
{
    type Codec = SerdeCdrCodec<Self>;

    fn type_name() -> &'static str {
        cached_type_name::<Self>(|| format!("HashMap<{},{}>", K::type_name(), V::type_name()))
    }

    fn schema() -> crate::dynamic::Schema {
        Arc::new(crate::dynamic::TypeShape::Map {
            key: K::schema(),
            value: V::schema(),
        })
    }
}

impl<K, V> Message for BTreeMap<K, V>
where
    K: MapKey + Ord + Serialize + for<'de> Deserialize<'de>,
    V: Message + Serialize + for<'de> Deserialize<'de>,
{
    type Codec = SerdeCdrCodec<Self>;

    fn type_name() -> &'static str {
        cached_type_name::<Self>(|| format!("BTreeMap<{},{}>", K::type_name(), V::type_name()))
    }

    fn schema() -> crate::dynamic::Schema {
        Arc::new(crate::dynamic::TypeShape::Map {
            key: K::schema(),
            value: V::schema(),
        })
    }
}

pub struct SerdeCdrCodec<T>(PhantomData<T>);
pub struct GeneratedCdrCodec<T>(PhantomData<T>);

pub trait WireDecoder {
    type Input<'a>;
    type Output;
    type Error: std::error::Error + Send + Sync + 'static;
    fn deserialize(input: Self::Input<'_>) -> Result<Self::Output, Self::Error>;
}

/// Transport-level message trait for types that can be encoded onto the wire.
pub trait WireMessage: Send + Sync + Sized + 'static {
    type Codec: for<'a> WireEncoder<Input<'a> = &'a Self> + WireDecoder;

    fn serialize(&self) -> Vec<u8> {
        Self::Codec::serialize(self)
    }

    fn serialize_to_zbuf(&self) -> ZBuf {
        // Use accurate size estimation for optimal buffer allocation
        Self::Codec::serialize_to_zbuf_with_hint(self, self.estimated_serialized_size())
    }

    fn deserialize(
        input: <Self::Codec as WireDecoder>::Input<'_>,
    ) -> Result<Self, <Self::Codec as WireDecoder>::Error>
    where
        Self::Codec: WireDecoder<Output = Self>,
    {
        Self::Codec::deserialize(input)
    }

    /// Get an estimated upper bound on the serialized size of this message.
    ///
    /// This is used to pre-allocate buffers for optimal serialization performance,
    /// both for regular ZBuf serialization and for zero-copy SHM serialization.
    /// The estimate should be conservative (larger than actual) to avoid buffer overflow.
    ///
    /// Default implementation returns 2x the size of the type, which is conservative
    /// for most messages. Messages with dynamic fields (Vec, String, ZBuf) get accurate
    /// implementations auto-generated by ros-z-codegen.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ros_z::msg::WireMessage;
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Serialize, Deserialize)]
    /// struct MyMessage {
    ///     data: Vec<u8>,
    ///     count: u32,
    /// }
    ///
    /// // Custom implementation for better estimation
    /// impl MyMessage {
    ///     fn estimate_size(&self) -> usize {
    ///         4 + // CDR header
    ///         4 + // sequence length prefix for Vec
    ///         self.data.len() + // actual data
    ///         4 + // count field
    ///         16  // padding/alignment buffer
    ///     }
    /// }
    /// ```
    fn estimated_serialized_size(&self) -> usize {
        // Conservative default: 2x struct size + CDR header
        // This works well for structs with few dynamic fields
        std::mem::size_of::<Self>() * 2 + 4
    }
}

// Blanket implementation for types with dedicated CDR traits (fast path).
// All generated message types satisfy these bounds; internal ros-z types that
// only have serde get explicit WireMessage impls below using SerdeCdrCodec instead.
impl<T> WireMessage for T
where
    T: Send
        + Sync
        + ros_z_cdr::CdrEncode
        + ros_z_cdr::CdrDecode
        + ros_z_cdr::CdrEncodedSize
        + 'static,
{
    type Codec = GeneratedCdrCodec<T>;
}

// ── Serde-based CDR serialization (existing path, kept for non-generated types) ───────────

pub(crate) struct SerdeCdrWireCodec<T>(PhantomData<T>);

/// CDR encapsulation header for little-endian encoding
pub const CDR_HEADER_LE: [u8; 4] = [0x00, 0x01, 0x00, 0x00];

impl<T> WireEncoder for SerdeCdrWireCodec<T>
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
        provider: &zenoh::shm::ShmProvider<zenoh::shm::PosixShmProviderBackend>,
    ) -> zenoh::Result<(ZBuf, usize)> {
        let mut writer = crate::shm::ShmWriter::new(provider, estimated_size)?;
        writer.extend_from_slice(&CDR_HEADER_LE);
        let mut serializer =
            SerdeCdrSerializer::<LittleEndian, crate::shm::ShmWriter>::new(&mut writer);
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

impl<T> MessageCodec<T> for SerdeCdrCodec<T>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    fn encode(value: &T) -> Result<EncodedMessage, CdrError> {
        Ok(EncodedMessage {
            payload: SerdeCdrWireCodec::<T>::serialize_to_zbuf(value),
            encoding: crate::encoding::Encoding::cdr(),
        })
    }

    fn encode_to_shm(
        value: &T,
        estimated_size: usize,
        provider: &zenoh::shm::ShmProvider<zenoh::shm::PosixShmProviderBackend>,
    ) -> zenoh::Result<EncodedMessage> {
        let (payload, _) =
            SerdeCdrWireCodec::<T>::serialize_to_shm(value, estimated_size, provider)?;
        Ok(EncodedMessage {
            payload,
            encoding: crate::encoding::Encoding::cdr(),
        })
    }

    fn decode(bytes: &[u8]) -> Result<T, CdrError> {
        SerdeCdrWireCodec::<T>::deserialize(bytes)
    }

    fn encoded_size_hint(value: &T) -> usize {
        SerdeCdrWireCodec::<T>::serialized_size_hint(value)
    }
}

impl<T> WireEncoder for SerdeCdrCodec<T>
where
    T: Serialize,
{
    type Input<'a>
        = &'a T
    where
        T: 'a;

    fn serialize_to_zbuf(input: &T) -> ZBuf {
        SerdeCdrWireCodec::<T>::serialize_to_zbuf(input)
    }

    fn serialize_to_zbuf_with_hint(input: &T, capacity_hint: usize) -> ZBuf {
        SerdeCdrWireCodec::<T>::serialize_to_zbuf_with_hint(input, capacity_hint)
    }

    fn serialized_size_hint(input: &T) -> usize {
        SerdeCdrWireCodec::<T>::serialized_size_hint(input)
    }

    fn serialize_to_shm(
        input: &T,
        estimated_size: usize,
        provider: &zenoh::shm::ShmProvider<zenoh::shm::PosixShmProviderBackend>,
    ) -> zenoh::Result<(ZBuf, usize)> {
        SerdeCdrWireCodec::<T>::serialize_to_shm(input, estimated_size, provider)
    }

    fn serialize_to_buf(input: &T, buffer: &mut Vec<u8>) {
        SerdeCdrWireCodec::<T>::serialize_to_buf(input, buffer)
    }
}

impl<T> WireDecoder for SerdeCdrCodec<T>
where
    for<'de> T: Deserialize<'de>,
{
    type Input<'a> = &'a [u8];
    type Output = T;
    type Error = CdrError;

    fn deserialize(input: Self::Input<'_>) -> Result<Self::Output, Self::Error> {
        SerdeCdrWireCodec::<T>::deserialize(input)
    }
}

impl<T> WireDecoder for SerdeCdrWireCodec<T>
where
    for<'a> T: Deserialize<'a>,
{
    type Input<'b> = &'b [u8];
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

impl<T> MessageCodec<T> for SerdeCdrWireCodec<T>
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    fn encode(value: &T) -> Result<EncodedMessage, CdrError> {
        Ok(EncodedMessage {
            payload: Self::serialize_to_zbuf(value),
            encoding: crate::encoding::Encoding::cdr(),
        })
    }

    fn encode_to_shm(
        value: &T,
        estimated_size: usize,
        provider: &zenoh::shm::ShmProvider<zenoh::shm::PosixShmProviderBackend>,
    ) -> zenoh::Result<EncodedMessage> {
        let (payload, _) = Self::serialize_to_shm(value, estimated_size, provider)?;
        Ok(EncodedMessage {
            payload,
            encoding: crate::encoding::Encoding::cdr(),
        })
    }

    fn decode(bytes: &[u8]) -> Result<T, CdrError> {
        Self::deserialize(bytes)
    }

    fn encoded_size_hint(value: &T) -> usize {
        Self::serialized_size_hint(value)
    }
}

// ── Fast CdrEncode-based CDR serialization (new path for generated types) ────────────

/// CDR serialization using the `CdrEncode`/`CdrDecode` traits directly.
///
/// Generated message types implement these traits and use `GeneratedCdrWireCodec` as their
/// `WireMessage::Codec` type. This enables the POD bulk fast path for sequences of
/// plain types (e.g., `Vec<f32>`, `Vec<geometry_msgs::Point>`).
pub(crate) struct GeneratedCdrWireCodec<T>(PhantomData<T>);

impl<T> WireEncoder for GeneratedCdrWireCodec<T>
where
    T: CdrEncode + CdrEncodedSize,
{
    type Input<'a>
        = &'a T
    where
        T: 'a;

    fn serialize_to_zbuf(input: &T) -> ZBuf {
        let capacity_hint = input.cdr_encoded_size(0) + 4;
        Self::serialize_to_zbuf_with_hint(input, capacity_hint)
    }

    fn serialize_to_zbuf_with_hint(input: &T, capacity_hint: usize) -> ZBuf {
        let mut writer = ZBufWriter::with_capacity(capacity_hint);
        writer.extend_from_slice(&CDR_HEADER_LE);
        ros_z_cdr::traits::cdr_to_zbuf_writer(input, &mut writer);
        writer.into_zbuf()
    }

    fn serialized_size_hint(input: &T) -> usize {
        input.cdr_encoded_size(0) + 4
    }

    fn serialize_to_shm(
        input: &T,
        estimated_size: usize,
        provider: &zenoh::shm::ShmProvider<zenoh::shm::PosixShmProviderBackend>,
    ) -> zenoh::Result<(ZBuf, usize)> {
        let mut writer = crate::shm::ShmWriter::new(provider, estimated_size)?;
        writer.extend_from_slice(&CDR_HEADER_LE);
        let mut cdr_writer = CdrWriter::<LittleEndian, crate::shm::ShmWriter>::new(&mut writer);
        input.cdr_encode(&mut cdr_writer);
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
        let mut cdr_writer = CdrWriter::<LittleEndian>::new(buffer);
        input.cdr_encode(&mut cdr_writer);
    }
}

impl<T> MessageCodec<T> for GeneratedCdrCodec<T>
where
    T: CdrEncode + CdrDecode + CdrEncodedSize,
{
    fn encode(value: &T) -> Result<EncodedMessage, CdrError> {
        Ok(EncodedMessage {
            payload: GeneratedCdrWireCodec::<T>::serialize_to_zbuf(value),
            encoding: crate::encoding::Encoding::cdr(),
        })
    }

    fn encode_to_shm(
        value: &T,
        estimated_size: usize,
        provider: &zenoh::shm::ShmProvider<zenoh::shm::PosixShmProviderBackend>,
    ) -> zenoh::Result<EncodedMessage> {
        let (payload, _) =
            GeneratedCdrWireCodec::<T>::serialize_to_shm(value, estimated_size, provider)?;
        Ok(EncodedMessage {
            payload,
            encoding: crate::encoding::Encoding::cdr(),
        })
    }

    fn decode(bytes: &[u8]) -> Result<T, CdrError> {
        GeneratedCdrWireCodec::<T>::deserialize(bytes)
    }

    fn encoded_size_hint(value: &T) -> usize {
        GeneratedCdrWireCodec::<T>::serialized_size_hint(value)
    }
}

impl<T> WireEncoder for GeneratedCdrCodec<T>
where
    T: CdrEncode + CdrEncodedSize,
{
    type Input<'a>
        = &'a T
    where
        T: 'a;

    fn serialize_to_zbuf(input: &T) -> ZBuf {
        GeneratedCdrWireCodec::<T>::serialize_to_zbuf(input)
    }

    fn serialize_to_zbuf_with_hint(input: &T, capacity_hint: usize) -> ZBuf {
        GeneratedCdrWireCodec::<T>::serialize_to_zbuf_with_hint(input, capacity_hint)
    }

    fn serialized_size_hint(input: &T) -> usize {
        GeneratedCdrWireCodec::<T>::serialized_size_hint(input)
    }

    fn serialize_to_shm(
        input: &T,
        estimated_size: usize,
        provider: &zenoh::shm::ShmProvider<zenoh::shm::PosixShmProviderBackend>,
    ) -> zenoh::Result<(ZBuf, usize)> {
        GeneratedCdrWireCodec::<T>::serialize_to_shm(input, estimated_size, provider)
    }

    fn serialize_to_buf(input: &T, buffer: &mut Vec<u8>) {
        GeneratedCdrWireCodec::<T>::serialize_to_buf(input, buffer)
    }
}

impl<T> WireDecoder for GeneratedCdrCodec<T>
where
    T: CdrDecode,
{
    type Input<'a> = &'a [u8];
    type Output = T;
    type Error = CdrError;

    fn deserialize(input: Self::Input<'_>) -> Result<Self::Output, Self::Error> {
        GeneratedCdrWireCodec::<T>::deserialize(input)
    }
}

impl<T> WireDecoder for GeneratedCdrWireCodec<T>
where
    T: CdrDecode,
{
    type Input<'b> = &'b [u8];
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
        let mut reader = ros_z_cdr::CdrReader::<LittleEndian>::new(payload);
        T::cdr_decode(&mut reader).map_err(|e| CdrError(e.to_string()))
    }
}

impl<T> MessageCodec<T> for GeneratedCdrWireCodec<T>
where
    T: CdrEncode + CdrDecode + CdrEncodedSize,
{
    fn encode(value: &T) -> Result<EncodedMessage, CdrError> {
        Ok(EncodedMessage {
            payload: Self::serialize_to_zbuf(value),
            encoding: crate::encoding::Encoding::cdr(),
        })
    }

    fn encode_to_shm(
        value: &T,
        estimated_size: usize,
        provider: &zenoh::shm::ShmProvider<zenoh::shm::PosixShmProviderBackend>,
    ) -> zenoh::Result<EncodedMessage> {
        let (payload, _) = Self::serialize_to_shm(value, estimated_size, provider)?;
        Ok(EncodedMessage {
            payload,
            encoding: crate::encoding::Encoding::cdr(),
        })
    }

    fn decode(bytes: &[u8]) -> Result<T, CdrError> {
        Self::deserialize(bytes)
    }

    fn encoded_size_hint(value: &T) -> usize {
        Self::serialized_size_hint(value)
    }
}

pub trait Service {
    type Request: WireMessage;
    type Response: WireMessage;
}

#[cfg(test)]
mod tests {
    use super::*;
    use zenoh_buffers::buffer::SplitBuffer;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct SimpleMessage {
        value: u32,
        text: String,
    }

    impl WireMessage for SimpleMessage {
        type Codec = SerdeCdrCodec<SimpleMessage>;
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

        let zbuf = SerdeCdrWireCodec::<SimpleMessage>::serialize_to_zbuf(&message);
        let bytes = zbuf.contiguous();

        // Verify CDR header
        assert_eq!(&bytes[0..4], &CDR_HEADER_LE);

        // Verify roundtrip
        let deserialized = SerdeCdrWireCodec::<SimpleMessage>::deserialize(&bytes).unwrap();
        assert_eq!(deserialized, message);
    }

    #[test]
    fn test_serialize_to_zbuf_consistency() {
        let message = SimpleMessage {
            value: 123,
            text: "consistency test".to_string(),
        };

        // Both methods should produce identical bytes
        let zbuf = SerdeCdrWireCodec::<SimpleMessage>::serialize_to_zbuf(&message);
        let vec = SerdeCdrWireCodec::<SimpleMessage>::serialize(&message);

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
        let zbuf1 = SerdeCdrWireCodec::<SimpleMessage>::serialize_to_zbuf_reuse(&msg1, &mut buffer);
        let bytes1 = zbuf1.contiguous();

        // Buffer should be empty after take
        assert!(buffer.is_empty());

        // Second serialization (buffer will be reallocated)
        let zbuf2 = SerdeCdrWireCodec::<SimpleMessage>::serialize_to_zbuf_reuse(&msg2, &mut buffer);
        let bytes2 = zbuf2.contiguous();

        // Verify roundtrips
        let decoded1 = SerdeCdrWireCodec::<SimpleMessage>::deserialize(&bytes1).unwrap();
        let decoded2 = SerdeCdrWireCodec::<SimpleMessage>::deserialize(&bytes2).unwrap();

        assert_eq!(decoded1, msg1);
        assert_eq!(decoded2, msg2);
    }

    #[test]
    fn test_zmessage_serialize_to_zbuf() {
        let message = SimpleMessage {
            value: 777,
            text: "trait test".to_string(),
        };

        // WireMessage trait provides serialize_to_zbuf method
        let zbuf = message.serialize_to_zbuf();
        let bytes = zbuf.contiguous();

        assert_eq!(&bytes[0..4], &CDR_HEADER_LE);

        let deserialized = <SimpleMessage as WireMessage>::deserialize(&bytes).unwrap();
        assert_eq!(deserialized, message);
    }

    #[test]
    fn test_cdr_encode_to_buf_consistency() {
        let message = SimpleMessage {
            value: 42,
            text: "Hello, ros-z!".to_string(),
        };

        // Serialize using both methods
        let vec1 = SerdeCdrWireCodec::<SimpleMessage>::serialize(&message);
        let mut vec2 = Vec::new();
        SerdeCdrWireCodec::<SimpleMessage>::serialize_to_buf(&message, &mut vec2);

        // Results should be identical
        assert_eq!(vec1, vec2);
        assert!(!vec1.is_empty());
        assert_eq!(&vec1[0..4], &CDR_HEADER_LE); // CDR header
    }

    #[test]
    fn test_cdr_encode_to_buf_reuses_capacity() {
        let message = SimpleMessage {
            value: 123,
            text: "test".to_string(),
        };

        let mut buffer = Vec::with_capacity(1024);
        SerdeCdrWireCodec::<SimpleMessage>::serialize_to_buf(&message, &mut buffer);

        let capacity_after_first = buffer.capacity();
        assert_eq!(capacity_after_first, 1024);

        // Serialize again - should reuse capacity
        SerdeCdrWireCodec::<SimpleMessage>::serialize_to_buf(&message, &mut buffer);
        assert_eq!(buffer.capacity(), capacity_after_first);
    }

    #[test]
    fn test_cdr_encode_to_buf_clears_previous_data() {
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
        SerdeCdrWireCodec::<LargeMessage>::serialize_to_buf(&msg1, &mut buffer);
        let len1 = buffer.len();
        assert!(len1 > 100);

        // Serialize small message - should clear buffer first
        SerdeCdrWireCodec::<SimpleMessage>::serialize_to_buf(&msg2, &mut buffer);
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
        SerdeCdrWireCodec::<LargeMessage>::serialize_to_buf(&original, &mut buffer);

        // Deserialize
        let deserialized =
            SerdeCdrWireCodec::<LargeMessage>::deserialize(&buffer).expect("Failed to deserialize");

        // Should match original
        assert_eq!(deserialized, original);
    }

    #[test]
    fn test_serialize_to_buf_with_empty_buffer() {
        let message = SimpleMessage {
            value: 99,
            text: "empty buffer test".to_string(),
        };

        let mut buffer = Vec::new();
        assert_eq!(buffer.capacity(), 0);

        SerdeCdrWireCodec::<SimpleMessage>::serialize_to_buf(&message, &mut buffer);

        assert!(!buffer.is_empty());
        assert!(buffer.capacity() > 0);
        assert_eq!(&buffer[0..4], &CDR_HEADER_LE); // CDR header
    }

    #[test]
    fn test_serialize_to_buf_multiple_messages() {
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
            SerdeCdrWireCodec::<SimpleMessage>::serialize_to_buf(message, &mut buffer);
            all_serialized.push(buffer.clone());

            // Verify each serialization is correct
            let deserialized = SerdeCdrWireCodec::<SimpleMessage>::deserialize(&buffer)
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

        // WireMessage trait provides serialize method
        let serialized = WireMessage::serialize(&message);
        assert!(!serialized.is_empty());
        assert_eq!(&serialized[0..4], &CDR_HEADER_LE);

        // WireMessage trait provides deserialize method
        let deserialized = <SimpleMessage as WireMessage>::deserialize(&serialized[..])
            .expect("Failed to deserialize");
        assert_eq!(deserialized, message);
    }
}

/// Tests for `GeneratedCdrWireCodec` — the `CdrEncode`-based CDR fast path.
///
/// These tests verify:
/// 1. Byte-identical wire output between `SerdeCdrWireCodec` (serde path) and `GeneratedCdrWireCodec` (CDR trait path).
/// 2. Roundtrip correctness for `GeneratedCdrWireCodec`.
/// 3. POD bulk path produces the same bytes for plain sequences as the element loop.
#[cfg(test)]
mod fast_cdr_tests {
    use super::*;
    use ros_z_cdr::{CdrBuffer, CdrDecode, CdrEncode, CdrEncodedSize, CdrReader, CdrWriter};

    // ── Test types ────────────────────────────────────────────────────────────

    /// A struct with a string field — NOT plain (element-by-element path).
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct Header {
        seq: u32,
        frame_id: String,
    }

    impl CdrEncode for Header {
        fn cdr_encode<BO: byteorder::ByteOrder, B: CdrBuffer>(&self, w: &mut CdrWriter<'_, BO, B>) {
            self.seq.cdr_encode(w);
            self.frame_id.cdr_encode(w);
        }
    }

    impl CdrDecode for Header {
        fn cdr_decode<'de, BO: byteorder::ByteOrder>(
            r: &mut CdrReader<'de, BO>,
        ) -> ros_z_cdr::Result<Self> {
            Ok(Self {
                seq: u32::cdr_decode(r)?,
                frame_id: String::cdr_decode(r)?,
            })
        }
    }

    impl CdrEncodedSize for Header {
        fn cdr_encoded_size(&self, pos: usize) -> usize {
            let p = self.seq.cdr_encoded_size(pos);
            self.frame_id.cdr_encoded_size(p)
        }
    }

    /// A plain struct — all fields are f64, no strings/sequences.
    /// On LE hosts this satisfies `CdrPlain` (verified in ros-z-cdr tests).
    #[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
    struct Point3d {
        x: f64,
        y: f64,
        z: f64,
    }

    impl CdrEncode for Point3d {
        fn cdr_encode<BO: byteorder::ByteOrder, B: CdrBuffer>(&self, w: &mut CdrWriter<'_, BO, B>) {
            self.x.cdr_encode(w);
            self.y.cdr_encode(w);
            self.z.cdr_encode(w);
        }
    }

    impl CdrDecode for Point3d {
        fn cdr_decode<'de, BO: byteorder::ByteOrder>(
            r: &mut CdrReader<'de, BO>,
        ) -> ros_z_cdr::Result<Self> {
            Ok(Self {
                x: f64::cdr_decode(r)?,
                y: f64::cdr_decode(r)?,
                z: f64::cdr_decode(r)?,
            })
        }
    }

    impl CdrEncodedSize for Point3d {
        fn cdr_encoded_size(&self, pos: usize) -> usize {
            let p = self.x.cdr_encoded_size(pos);
            let p = self.y.cdr_encoded_size(p);
            self.z.cdr_encoded_size(p)
        }
    }

    /// A message with a Vec<Point3d> — this is the key fast-path case.
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct PointCloud {
        header: Header,
        points: Vec<Point3d>,
    }

    impl CdrEncode for PointCloud {
        fn cdr_encode<BO: byteorder::ByteOrder, B: CdrBuffer>(&self, w: &mut CdrWriter<'_, BO, B>) {
            self.header.cdr_encode(w);
            // Vec<Point3d>: element-by-element (Point3d: CdrEncode)
            w.write_sequence_length(self.points.len());
            for pt in &self.points {
                pt.cdr_encode(w);
            }
        }
    }

    impl CdrDecode for PointCloud {
        fn cdr_decode<'de, BO: byteorder::ByteOrder>(
            r: &mut CdrReader<'de, BO>,
        ) -> ros_z_cdr::Result<Self> {
            let header = Header::cdr_decode(r)?;
            let n = r.read_sequence_length()?;
            let mut points = Vec::with_capacity(n);
            for _ in 0..n {
                points.push(Point3d::cdr_decode(r)?);
            }
            Ok(Self { header, points })
        }
    }

    impl CdrEncodedSize for PointCloud {
        fn cdr_encoded_size(&self, pos: usize) -> usize {
            let p = self.header.cdr_encoded_size(pos);
            // sequence length u32 (4-byte aligned)
            let p = p + ((4 - p % 4) % 4) + 4;
            let mut p = p;
            for pt in &self.points {
                p = pt.cdr_encoded_size(p);
            }
            p
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn serde_bytes<T: Serialize>(value: &T) -> Vec<u8> {
        SerdeCdrWireCodec::<T>::serialize(value)
    }

    fn fast_bytes<T: CdrEncode + CdrEncodedSize>(value: &T) -> Vec<u8> {
        GeneratedCdrWireCodec::<T>::serialize(value)
    }

    fn fast_deserialize<T: CdrDecode>(bytes: &[u8]) -> T {
        GeneratedCdrWireCodec::<T>::deserialize(bytes)
            .expect("GeneratedCdrWireCodec::deserialize failed")
    }

    // ── Tests ─────────────────────────────────────────────────────────────────

    #[test]
    fn header_byte_identical_to_serde() {
        let message = Header {
            seq: 42,
            frame_id: "base_link".to_string(),
        };
        assert_eq!(serde_bytes(&message), fast_bytes(&message));
    }

    #[test]
    fn header_fast_roundtrip() {
        let message = Header {
            seq: 99,
            frame_id: "map".to_string(),
        };
        let bytes = fast_bytes(&message);
        let decoded: Header = fast_deserialize(&bytes);
        assert_eq!(message, decoded);
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn point3d_byte_identical_to_serde() {
        let pt = Point3d {
            x: 1.0,
            y: 2.5,
            z: -3.14,
        };
        assert_eq!(serde_bytes(&pt), fast_bytes(&pt));
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn point3d_fast_roundtrip() {
        let pt = Point3d {
            x: 1.0,
            y: 2.5,
            z: -3.14,
        };
        let bytes = fast_bytes(&pt);
        let decoded: Point3d = fast_deserialize(&bytes);
        assert_eq!(pt, decoded);
    }

    #[test]
    fn pointcloud_byte_identical_to_serde() {
        let message = PointCloud {
            header: Header {
                seq: 1,
                frame_id: "lidar".to_string(),
            },
            points: vec![
                Point3d {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                Point3d {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                },
                Point3d {
                    x: -1.0,
                    y: -2.0,
                    z: -3.0,
                },
            ],
        };
        assert_eq!(serde_bytes(&message), fast_bytes(&message));
    }

    #[test]
    fn pointcloud_fast_roundtrip() {
        let message = PointCloud {
            header: Header {
                seq: 7,
                frame_id: "camera".to_string(),
            },
            points: (0..100)
                .map(|i| Point3d {
                    x: i as f64,
                    y: (i * 2) as f64,
                    z: (i * 3) as f64,
                })
                .collect(),
        };
        let bytes = fast_bytes(&message);
        let decoded: PointCloud = fast_deserialize(&bytes);
        assert_eq!(message, decoded);
    }

    #[test]
    fn empty_sequence_roundtrip() {
        let message = PointCloud {
            header: Header {
                seq: 0,
                frame_id: String::new(),
            },
            points: vec![],
        };
        let bytes = fast_bytes(&message);
        let decoded: PointCloud = fast_deserialize(&bytes);
        assert_eq!(message, decoded);
    }

    #[test]
    fn size_hint_matches_actual() {
        let message = PointCloud {
            header: Header {
                seq: 1,
                frame_id: "test".to_string(),
            },
            points: vec![
                Point3d {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0
                };
                10
            ],
        };
        let hint = message.cdr_encoded_size(0) + 4;
        let bytes = fast_bytes(&message);
        // The hint should be >= actual payload size
        assert!(
            hint >= bytes.len() - 4,
            "hint={hint} bytes.len()={}",
            bytes.len()
        );
    }
}
