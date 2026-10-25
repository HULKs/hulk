use ros_z::{
    CdrCodec, Message, ZSlice,
    context::ContextBuilder,
    message::{WireDecoder, WireEncoder},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::Arc,
    time::Duration,
};
use zenoh_buffers::{ZBuf, buffer::SplitBuffer};

#[test]
fn numeric_collections_use_compatible_bulk_layouts() {
    fn check<T: Message + Serialize + PartialEq + std::fmt::Debug>(values: T) {
        #[derive(Debug, PartialEq, Serialize, Message)]
        struct Aligned<T> {
            prefix: u8,
            values: T,
            suffix: u64,
        }
        let value = Aligned {
            prefix: 3,
            values,
            suffix: 99,
        };
        let wire = CdrCodec::<Aligned<T>>::serialize(&value).unwrap();
        assert_eq!(wire, serde_wire(&value));
        assert_eq!(CdrCodec::<Aligned<T>>::deserialize(&wire).unwrap(), value);
        let mut buffer = ZBuf::empty();
        for part in wire.chunks(3) {
            buffer.push_zslice(part.to_vec().into());
        }
        assert_eq!(
            CdrCodec::<Aligned<T>>::deserialize_payload(&buffer.into()).unwrap(),
            value
        );
        if wire.len() > 4 {
            assert!(CdrCodec::<Aligned<T>>::deserialize(&wire[..wire.len() - 1]).is_err());
        }
    }
    macro_rules! check_numeric {
        ($($ty:ty),*) => { $(
            check(vec![1 as $ty, 2 as $ty, 3 as $ty]);
            check([1 as $ty, 2 as $ty, 3 as $ty]);
            check(Vec::<$ty>::new());
            check([] as [$ty; 0]);
        )* };
    }
    check_numeric!(u8, i8, u16, i16, u32, i32, u64, i64, f32, f64);

    #[derive(Default)]
    struct Writes {
        bytes: usize,
        bulk_writes: usize,
    }
    impl ros_z_cdr::CdrBuffer for Writes {
        fn extend_from_slice(&mut self, bytes: &[u8]) {
            self.bytes += bytes.len();
            self.bulk_writes += 1;
        }
        fn push(&mut self, _: u8) {
            self.bytes += 1;
        }
        fn len(&self) -> usize {
            self.bytes
        }
        fn clear(&mut self) {
            *self = Self::default();
        }
    }
    let mut writes = Writes::default();
    vec![1u32; 1024]
        .encode_cdr(&mut ros_z_cdr::CdrWriter::new(&mut writes))
        .unwrap();
    #[cfg(target_endian = "little")]
    assert_eq!(writes.bulk_writes, 2); // Length prefix and one bulk payload write.
}

#[test]
fn shm_reuses_the_counted_size_without_another_traversal() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static TRAVERSALS: AtomicUsize = AtomicUsize::new(0);

    #[derive(Deserialize, Message)]
    #[message(serde)]
    struct Counted {
        value: u32,
    }

    impl Serialize for Counted {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            TRAVERSALS.fetch_add(1, Ordering::Relaxed);
            self.value.serialize(serializer)
        }
    }

    let provider = ros_z::shm::ShmProviderBuilder::new(64 * 1024)
        .build()
        .unwrap();
    let value = Counted { value: 42 };
    let hint = CdrCodec::<Counted>::serialized_size_hint(&value);
    assert_eq!(TRAVERSALS.load(Ordering::Relaxed), 1);
    let (buffer, size) = CdrCodec::<Counted>::serialize_to_shm(&value, hint, &provider).unwrap();
    assert_eq!(size, hint);
    assert_eq!(TRAVERSALS.load(Ordering::Relaxed), 2);
    assert_eq!(
        CdrCodec::<Counted>::deserialize_payload(&buffer.into())
            .unwrap()
            .value,
        42
    );
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Message)]
struct Bytes {
    prefix: u8,
    #[serde(with = "ros_z::serde_zslice")]
    bytes: ZSlice,
    suffix: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Message)]
struct Stereo {
    sequence: u32,
    left: Bytes,
    right: Bytes,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Message)]
enum Choice<T> {
    Empty,
    Newtype(T),
    Tuple(u8, T),
    Named { flag: bool, value: T },
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Message)]
struct Generic<T> {
    prefix: u8,
    value: T,
    tail: u64,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Message)]
#[serde(transparent)]
struct Transparent(u64);

#[derive(Debug, PartialEq, Serialize, Deserialize, Message)]
struct Skipped {
    prefix: u8,
    #[serde(skip)]
    omitted: u32,
    suffix: u64,
}

#[derive(Debug, PartialEq, Message)]
#[message(serde)]
struct Custom(u64);
impl Serialize for Custom {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u64(self.0 ^ 0x55)
    }
}
impl<'de> Deserialize<'de> for Custom {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        u64::deserialize(deserializer).map(|n| Self(n ^ 0x55))
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Foreign(u64);
impl ros_z::MessageSchema for Foreign {
    fn build_schema(
        builder: &mut ros_z::SchemaBuilder,
    ) -> Result<ros_z_schema::TypeDef, ros_z_schema::SchemaError> {
        <u64 as ros_z::MessageSchema>::build_schema(builder)
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Message)]
struct Bridged<B> {
    prefix: u8,
    #[message(serde)]
    foreign: Foreign,
    value: B,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Message)]
#[serde(tag = "kind")]
enum Tagged {
    Named { n: u32 },
}

fn same_wire<T>(value: &T)
where
    T: Message + Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let old = serde_wire(value);
    let native = CdrCodec::<T>::serialize_to_zbuf(value).unwrap();
    assert_eq!(native.contiguous().as_ref(), old);
    assert_eq!(&CdrCodec::<T>::deserialize(&old).unwrap(), value);
    assert_eq!(
        &CdrCodec::<T>::deserialize_payload(&native.into()).unwrap(),
        value
    );
}

#[test]
fn wire_compatibility() {
    macro_rules! check { ($($value:expr),* $(,)?) => { $(same_wire(&Generic { prefix: 3, value: $value, tail: 0x123456789abcdef0 });)* }; }
    check!(
        true,
        -7i8,
        8u8,
        -13i16,
        14u16,
        -15i32,
        16u32,
        -17i64,
        18u64,
        19usize,
        1.25f32,
        -2.5f64,
        "hello".to_owned(),
        Duration::new(42, 9),
        std::time::UNIX_EPOCH + Duration::new(42, 9),
        std::path::PathBuf::from("/example"),
        "127.0.0.1:1234".parse::<std::net::SocketAddr>().unwrap()
    );
    check!(
        Some(3u64),
        None::<u64>,
        Ok::<u64, String>(42),
        Err::<u64, String>("failed".into()),
        vec![1u16, 2, 3],
        [1u64, 2, 3],
        Box::new(4u64),
        Arc::new(4u64),
        2u32..8,
        2u32..=8
    );
    check!(Arc::<[u8]>::from([1, 2, 3]), Box::<[u8]>::from([1, 2, 3]));
    check!(
        HashSet::from([1u32, 2, 3]),
        HashMap::from([(1u32, 4u64), (2, 5)]),
        BTreeMap::from([(1u32, 4u64), (2, 5)])
    );
    check!(
        Choice::<u64>::Empty,
        Choice::Newtype(9u64),
        Choice::Tuple(1, 9u64),
        Choice::Named {
            flag: true,
            value: 9u64
        },
        Transparent(17)
    );
    same_wire(&stereo());
    same_wire(&Skipped {
        prefix: 1,
        omitted: 0,
        suffix: 9,
    });
    same_wire(&Custom(42));
    let bridged = Bridged {
        prefix: 3,
        foreign: Foreign(0x12345678),
        value: stereo(),
    };
    same_wire(&bridged);
    let payload = CdrCodec::<Bridged<Stereo>>::serialize_to_zbuf(&bridged)
        .unwrap()
        .into();
    let decoded = CdrCodec::<Bridged<Stereo>>::deserialize_payload(&payload).unwrap();
    assert_eq!(
        decoded.value.left.bytes.as_ptr(),
        bridged.value.left.bytes.as_ptr()
    );
    let tagged = Tagged::Named { n: 17 };
    assert_eq!(
        <Tagged as Message>::Codec::serialize(&tagged).unwrap(),
        serde_wire(&tagged)
    );
    #[cfg(feature = "nalgebra")]
    check!(
        nalgebra::Vector3::new(1.0f32, 2.0, 3.0),
        nalgebra::Isometry3::<f64>::identity()
    );
    #[cfg(feature = "ndarray")]
    check!(ndarray::arr2(&[[1.0f32, 2.0], [3.0, 4.0]]));
}

fn serde_wire<T: Serialize>(value: &T) -> Vec<u8> {
    let mut bytes = ros_z::message::CDR_HEADER_LE.to_vec();
    value
        .serialize(&mut ros_z_cdr::SerdeCdrSerializer::<byteorder::LittleEndian>::new(&mut bytes))
        .unwrap();
    bytes
}

fn stereo() -> Stereo {
    Stereo {
        sequence: 12,
        left: Bytes {
            prefix: 5,
            bytes: vec![0x52; 1024 * 1024].into(),
            suffix: 0x123456789abcdef0,
        },
        right: Bytes {
            prefix: 6,
            bytes: vec![0xa7; 1024 * 1024].into(),
            suffix: 77,
        },
    }
}

#[test]
fn encode_decode_share_pixels_and_retain_owners() {
    let value = stereo();
    let left = value.left.bytes.as_ptr();
    let right = value.right.bytes.as_ptr();
    let buffer = CdrCodec::<Stereo>::serialize_to_zbuf(&value).unwrap();
    assert!(buffer.zslices().any(|s| s.as_ptr() == left));
    assert!(buffer.zslices().any(|s| s.as_ptr() == right));
    let payload = buffer.into();
    let decoded = CdrCodec::<Stereo>::deserialize_payload(&payload).unwrap();
    assert_eq!(decoded.left.bytes.as_ptr(), left);
    assert_eq!(decoded.right.bytes.as_ptr(), right);
    drop(payload);
    drop(value);
    assert_eq!(decoded.left.bytes[1024 * 1024 - 1], 0x52);
    assert_eq!(decoded.right.bytes[1024 * 1024 - 1], 0xa7);
}

#[test]
fn explicit_shm_does_not_trust_size_estimates() {
    let value = Bytes {
        prefix: 3,
        bytes: vec![42; 1024].into(),
        suffix: 99,
    };
    let wire = CdrCodec::<Bytes>::serialize(&value).unwrap();
    let provider = ros_z::shm::ShmProviderBuilder::new(1024 * 1024)
        .build()
        .unwrap();
    for estimate in [1, wire.len() + 128] {
        let (buffer, size) =
            CdrCodec::<Bytes>::serialize_to_shm(&value, estimate, &provider).unwrap();
        assert_eq!(size, wire.len());
        assert_eq!(buffer.contiguous().as_ref(), wire);
        assert_eq!(
            CdrCodec::<Bytes>::deserialize_payload(&buffer.into()).unwrap(),
            value
        );
    }
}

#[test]
fn arbitrary_segmentation_and_empty_fields() {
    for bytes in [vec![], vec![1, 2, 3, 4, 5, 6, 7]] {
        let value = Bytes {
            prefix: 3,
            bytes: bytes.into(),
            suffix: 99,
        };
        let wire = CdrCodec::<Bytes>::serialize(&value).unwrap();
        for split in 0..=wire.len() {
            let mut buffer = ZBuf::empty();
            buffer.push_zslice(wire[..split].to_vec().into());
            buffer.push_zslice(wire[split..].to_vec().into());
            assert_eq!(
                CdrCodec::<Bytes>::deserialize_payload(&buffer.into()).unwrap(),
                value
            );
        }
        let mut buffer = ZBuf::empty();
        for byte in wire {
            buffer.push_zslice(vec![byte].into());
        }
        assert_eq!(
            CdrCodec::<Bytes>::deserialize_payload(&buffer.into()).unwrap(),
            value
        );
    }
}

#[test]
fn native_containers_do_not_require_serde_for_zslice() {
    #[derive(Message)]
    struct Container<T> {
        fields: Vec<Option<Arc<T>>>,
    }
    let value = Container {
        fields: vec![None, Some(Arc::new(ZSlice::from(vec![1, 2, 3])))],
    };
    let ptr = value.fields[1].as_ref().unwrap().as_ptr();
    let buffer = CdrCodec::<Container<ZSlice>>::serialize_to_zbuf(&value).unwrap();
    let decoded = CdrCodec::<Container<ZSlice>>::deserialize_payload(&buffer.into()).unwrap();
    assert_eq!(decoded.fields[1].as_ref().unwrap().as_ptr(), ptr);
}

#[test]
fn generic_foreign_bridges_and_recursive_fields() {
    #[derive(Debug, PartialEq, Serialize, Deserialize, Message)]
    struct FieldBridge<T> {
        prefix: u8,
        #[message(serde)]
        value: T,
        suffix: u64,
    }
    #[derive(Debug, PartialEq, Serialize, Deserialize, Message)]
    enum EnumBridge<T> {
        Value(#[message(serde)] T),
    }
    #[derive(Debug, PartialEq, Serialize, Deserialize, Message)]
    #[message(serde)]
    struct WholeBridge<B> {
        value: B,
    }
    #[derive(Debug, PartialEq, Serialize, Deserialize, Message)]
    struct Recursive<T> {
        value: T,
        children: Vec<Recursive<T>>,
    }
    #[derive(Debug, PartialEq, Serialize, Deserialize, Message)]
    enum Tree<T> {
        Leaf(T),
        Branch(Vec<Tree<T>>),
    }
    same_wire(&FieldBridge {
        prefix: 3,
        value: Foreign(5),
        suffix: 7,
    });
    same_wire(&EnumBridge::Value(Foreign(8)));
    same_wire(&WholeBridge { value: Foreign(9) });
    same_wire(&Recursive {
        value: 1u32,
        children: vec![Recursive {
            value: 2,
            children: vec![],
        }],
    });
    same_wire(&Tree::Branch(vec![Tree::Leaf(3u64)]));
    Recursive::<u32>::schema().validate().unwrap();
    Tree::<u64>::schema().validate().unwrap();
    // Wire attributes still select the same typed codec, only the field methods change.
    fn unified<T: Message<Codec = CdrCodec<T>>>() {}
    unified::<FieldBridge<Foreign>>();
    unified::<WholeBridge<Foreign>>();
    unified::<Skipped>();
    unified::<Tagged>();
}

#[test]
fn array_decode_is_empty_safe_and_drops_partial_values() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static DROPS: AtomicUsize = AtomicUsize::new(0);
    #[derive(Message)]
    struct Dropped {
        value: u8,
    }
    impl Drop for Dropped {
        fn drop(&mut self) {
            DROPS.fetch_add(1, Ordering::SeqCst);
        }
    }
    let mut wire = ros_z::message::CDR_HEADER_LE.to_vec();
    assert!(
        CdrCodec::<[Dropped; 0]>::deserialize(&wire)
            .unwrap()
            .is_empty()
    );
    assert_eq!(DROPS.load(Ordering::SeqCst), 0);
    wire.extend_from_slice(&[1, 2]);
    assert!(CdrCodec::<[Dropped; 3]>::deserialize(&wire).is_err());
    assert_eq!(DROPS.load(Ordering::SeqCst), 2);
    let values = CdrCodec::<[Dropped; 2]>::deserialize(&wire).unwrap();
    assert_eq!(values[0].value, 1);
    drop(values);
    assert_eq!(DROPS.load(Ordering::SeqCst), 4);
    let mut malformed = ros_z::message::CDR_HEADER_LE.to_vec();
    malformed.extend_from_slice(&100_000_000u32.to_le_bytes());
    assert!(CdrCodec::<Vec<u64>>::deserialize(&malformed).is_err());
}

#[test]
fn exact_size_hint_selects_large_native_messages() {
    let value = vec![17u32; 1024];
    assert_eq!(CdrCodec::<Vec<u32>>::serialized_size_hint(&value), 4104);
    let provider = ros_z::shm::ShmProviderBuilder::new(64 * 1024)
        .build()
        .unwrap();
    for estimate in [1, 1_000_000] {
        let (buffer, size) =
            CdrCodec::<Vec<u32>>::serialize_to_shm(&value, estimate, &provider).unwrap();
        assert_eq!(size, 4104);
        assert_eq!(buffer.contiguous().len(), size);
        assert_eq!(
            CdrCodec::<Vec<u32>>::deserialize(&buffer.contiguous()).unwrap(),
            value
        );
    }
}

#[test]
fn retained_shm_messages_exhaust_pool_without_stalling_publish() {
    // A separate OS thread bounds failure even if a synchronous allocator regresses to blocking.
    let (send, recv) = std::sync::mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        let result = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let provider = Arc::new(ros_z::shm::ShmProviderBuilder::new(32 * 1024).build()?);
                let context = ContextBuilder::default()
                    .disable_multicast_scouting()
                    .build()
                    .await?;
                let node = context.create_node("native_shm_retention").build().await?;
                let publisher = node
                    .publisher::<Bytes>("/retained_shm")
                    .shm_config(ros_z::shm::ShmConfig::new(provider.clone()))
                    .build()
                    .await?;
                let subscriber = node.subscriber::<Bytes>("/retained_shm").build().await?;
                let cache = node
                    .subscriber::<Bytes>("/retained_shm")
                    .cache(32)
                    .build()
                    .await?;
                let stamped = node
                    .subscriber::<Bytes>("/retained_shm")
                    .cache(32)
                    .with_stamp(|value: &Bytes| ros_z::time::Time::from_nanos(value.suffix as i64))
                    .build()
                    .await?;
                let mut value = Bytes {
                    prefix: 3,
                    bytes: vec![42; 8192].into(),
                    suffix: 0,
                };
                let mut retained = Vec::new();
                let mut saw_shm = false;
                let mut saw_fallback = false;
                for index in 1..=16 {
                    value.suffix = index;
                    publisher.publish(&value).await?;
                    let decoded = tokio::time::timeout(Duration::from_secs(5), subscriber.recv())
                        .await
                        .unwrap()?;
                    let shares_input = decoded.bytes.as_ptr() == value.bytes.as_ptr();
                    saw_shm |= !shares_input;
                    saw_fallback |= shares_input;
                    retained.push(decoded);
                    tokio::time::timeout(Duration::from_secs(5), async {
                        while cache.len() < retained.len() || stamped.len() < retained.len() {
                            tokio::task::yield_now().await;
                        }
                    })
                    .await
                    .unwrap();
                }
                assert!(
                    saw_shm,
                    "large native messages must cross the default SHM threshold"
                );
                assert!(
                    saw_fallback,
                    "retained allocations must trigger heap fallback"
                );
                assert_eq!(retained[0].bytes[8191], 42);
                let dynamic = ros_z::dynamic::DynamicPayload::new(
                    Arc::new(Vec::<u8>::schema()),
                    ros_z::dynamic::DynamicValue::Bytes(vec![42; 8192].into()),
                )
                .unwrap();
                assert!(
                    ros_z::dynamic::DynamicCdrCodec::serialize_to_shm(&dynamic, 1, &provider)
                        .is_err()
                );
                Ok::<_, ros_z::Error>(())
            });
        send.send(result).unwrap();
    });
    recv.recv_timeout(Duration::from_secs(20))
        .expect("SHM publishing stalled")
        .unwrap();
    worker.join().unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn publisher_selects_shm_for_large_native_vector() -> ros_z::Result<()> {
    #[derive(Message)]
    struct VectorMessage {
        values: Vec<u32>,
        marker: ZSlice,
    }
    let context = ContextBuilder::default()
        .disable_multicast_scouting()
        .build()
        .await?;
    let node = context.create_node("native_vector_shm").build().await?;
    let provider = Arc::new(ros_z::shm::ShmProviderBuilder::new(64 * 1024).build()?);
    let publisher = node
        .publisher::<VectorMessage>("/native_vector_shm")
        .shm_config(ros_z::shm::ShmConfig::new(provider))
        .build()
        .await?;
    let subscriber = node
        .subscriber::<VectorMessage>("/native_vector_shm")
        .build()
        .await?;
    let value = VectorMessage {
        values: vec![17; 1024],
        marker: vec![42].into(),
    };
    publisher.publish(&value).await?;
    let decoded = tokio::time::timeout(Duration::from_secs(5), subscriber.recv())
        .await
        .unwrap()?;
    assert_eq!(decoded.values, value.values);
    assert_eq!(decoded.marker, value.marker);
    // Heap encoding would share this tiny marker; only the vector crosses the threshold.
    assert_ne!(decoded.marker.as_ptr(), value.marker.as_ptr());
    Ok(())
}

#[test]
fn malformed_input_is_rejected() {
    fn wire(bytes: &[u8]) -> Vec<u8> {
        [ros_z::message::CDR_HEADER_LE.as_slice(), bytes].concat()
    }
    assert!(CdrCodec::<bool>::deserialize(&wire(&[2])).is_err());
    assert!(CdrCodec::<Option<u8>>::deserialize(&wire(&[2, 0, 0, 0])).is_err());
    assert!(CdrCodec::<Result<u8, String>>::deserialize(&wire(&[2, 0, 0, 0])).is_err());
    assert!(CdrCodec::<Choice<u8>>::deserialize(&wire(&[99, 0, 0, 0])).is_err());
    assert!(CdrCodec::<String>::deserialize(&wire(&[0, 0, 0, 0])).is_err());
    assert!(CdrCodec::<String>::deserialize(&wire(&[1, 0, 0, 0, 1])).is_err());
    assert!(CdrCodec::<String>::deserialize(&wire(&[2, 0, 0, 0, 0xff, 0])).is_err());
    assert!(CdrCodec::<Vec<u64>>::deserialize(&wire(&[0xff; 4])).is_err());
    assert!(CdrCodec::<[String; 3]>::deserialize(&wire(&[1, 0, 0, 0, 0])).is_err());
    assert!(CdrCodec::<ZSlice>::deserialize(&wire(&[0xff; 4])).is_err());
    assert!(CdrCodec::<ZSlice>::deserialize(&wire(&[99, 0, 0, 0])).is_err());
    assert!(CdrCodec::<u8>::deserialize(&[0, 0, 0, 0, 1]).is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_publish_two_subscribers_retain_pixel_pointers() -> ros_z::Result<()> {
    let context = ContextBuilder::default()
        .disable_multicast_scouting()
        .build()
        .await?;
    let node = context.create_node("native_cdr_sharing").build().await?;
    let publisher = node.publisher::<Stereo>("/native_stereo").build().await?;
    let first = node.subscriber::<Stereo>("/native_stereo").build().await?;
    let second = node.subscriber::<Stereo>("/native_stereo").build().await?;
    let cache = node
        .subscriber::<Stereo>("/native_stereo")
        .cache(2)
        .build()
        .await?;
    let extracted_cache = node
        .subscriber::<Stereo>("/native_stereo")
        .cache(2)
        .with_stamp(|value: &Stereo| ros_z::time::Time::from_nanos(value.sequence as i64))
        .build()
        .await?;
    let value = stereo();
    let left = value.left.bytes.as_ptr();
    let right = value.right.bytes.as_ptr();
    publisher.publish(&value).await?;
    let one = tokio::time::timeout(Duration::from_secs(5), first.recv())
        .await
        .unwrap()?;
    let two = tokio::time::timeout(Duration::from_secs(5), second.recv())
        .await
        .unwrap()?;
    assert_eq!(one.left.bytes.as_ptr(), left);
    assert_eq!(two.left.bytes.as_ptr(), left);
    assert_eq!(one.right.bytes.as_ptr(), right);
    assert_eq!(two.right.bytes.as_ptr(), right);
    tokio::time::timeout(Duration::from_secs(5), async {
        while cache.is_empty() || extracted_cache.is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(cache.get_latest().unwrap().left.bytes.as_ptr(), left);
    assert_eq!(
        extracted_cache.get_latest().unwrap().right.bytes.as_ptr(),
        right
    );
    drop(value);
    drop(publisher);
    drop(first);
    drop(second);
    drop(cache);
    drop(extracted_cache);
    drop(node);
    drop(context);
    assert_eq!(one.left.bytes[123], 0x52);
    assert_eq!(two.right.bytes[123], 0xa7);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn service_request_and_reply_retain_pixel_owners() -> ros_z::Result<()> {
    struct Echo;
    impl ros_z::message::Service for Echo {
        type Request = Stereo;
        type Response = Stereo;
    }
    impl ros_z::ServiceTypeInfo for Echo {
        fn service_type_info() -> ros_z::TypeInfo {
            Stereo::type_info()
        }
    }
    let context = ContextBuilder::default()
        .disable_multicast_scouting()
        .build()
        .await?;
    let node = context.create_node("native_cdr_echo").build().await?;
    let mut server = node.service_server::<Echo>("native_echo").build().await?;
    let client = node.service_client::<Echo>("native_echo").build().await?;
    let value = stereo();
    let pixels = value.left.bytes.as_ptr();
    let serve = async {
        let request = server.take_request_async().await?;
        assert_eq!(request.message().left.bytes.as_ptr(), pixels);
        let response = request.message().clone();
        request.reply_async(&response).await
    };
    let call = client.call_async(&value);
    let (served, response) =
        tokio::time::timeout(Duration::from_secs(5), async { tokio::join!(serve, call) })
            .await
            .unwrap();
    served?;
    let response = response?;
    assert_eq!(response.left.bytes.as_ptr(), pixels);
    drop(value);
    assert_eq!(response.left.bytes[123], 0x52);
    Ok(())
}
