use linear_algebra::{Framed, Transform};
use ros_z::{
    CdrCodec, Message, ZSlice,
    message::{WireDecoder, WireEncoder},
};

fn check_wrapper<T: Message>(value: &T, inner: &ZSlice, unwrap: impl FnOnce(T) -> ZSlice) {
    let wire = CdrCodec::<T>::serialize(value).unwrap();
    assert_eq!(wire, CdrCodec::<ZSlice>::serialize(inner).unwrap());
    assert_eq!(T::type_name(), ZSlice::type_name());
    assert_eq!(T::schema(), ZSlice::schema());
    assert_eq!(T::schema_hash(), ZSlice::schema_hash());
    let payload = CdrCodec::<T>::serialize_to_zbuf(value).unwrap().into();
    let decoded = unwrap(CdrCodec::<T>::deserialize_payload(&payload).unwrap());
    assert_eq!(decoded.as_ptr(), inner.as_ptr());
    drop(payload);
    assert_eq!(decoded.as_ref(), inner.as_ref());
}

#[test]
fn transparent_native_wrappers_need_no_serde_and_preserve_storage() {
    let framed = Framed::<(), _>::wrap(ZSlice::from(vec![1, 2, 3]));
    check_wrapper(&framed, &framed.inner, |value| value.inner);
    let transform = Transform::<(), (), _>::wrap(ZSlice::from(vec![4, 5, 6]));
    check_wrapper(&transform, &transform.inner, |value| value.inner);
}
