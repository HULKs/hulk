//! `CdrEncode`, `CdrDecode`, and `CdrEncodedSize` traits.
//!
//! These bypass serde for CDR serialization, enabling the bulk POD fast path
//! for sequences of plain types while keeping serde derives for non-CDR uses.

use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;

use byteorder::{ByteOrder, LittleEndian};

use crate::buffer::CdrBuffer;
use crate::error::Result;
use crate::primitives::{CdrReader, CdrWriter};
use crate::zbuf_writer::ZBufWriter;

// ── Core traits ──────────────────────────────────────────────────────────────

pub trait CdrEncode {
    fn cdr_encode<BO: ByteOrder, B: CdrBuffer>(&self, writer: &mut CdrWriter<'_, BO, B>);
}

pub trait CdrDecode: Sized {
    fn cdr_decode<'de, BO: ByteOrder>(reader: &mut CdrReader<'de, BO>) -> Result<Self>;
}

/// Size in bytes of the CDR-serialized form, given the current stream alignment position.
pub trait CdrEncodedSize {
    fn cdr_encoded_size(&self, current_alignment: usize) -> usize;
}

// ── Entry points ─────────────────────────────────────────────────────────────

/// Serialize `value` into a new `Vec<u8>` using CDR little-endian encoding.
pub fn cdr_to_vec<T: CdrEncode>(value: &T, capacity_hint: usize) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(capacity_hint);
    let mut writer = CdrWriter::<LittleEndian>::new(&mut buffer);
    value.cdr_encode(&mut writer);
    buffer
}

/// Serialize `value` into a `ZBufWriter` (for zero-copy Zenoh transport).
pub fn cdr_to_zbuf_writer<T: CdrEncode>(value: &T, writer: &mut ZBufWriter) {
    let mut cdr_writer = CdrWriter::<LittleEndian, ZBufWriter>::new(writer);
    value.cdr_encode(&mut cdr_writer);
}

// ── Primitive CdrEncode impls ─────────────────────────────────────────────

impl CdrEncode for bool {
    #[inline]
    fn cdr_encode<BO: ByteOrder, B: CdrBuffer>(&self, w: &mut CdrWriter<'_, BO, B>) {
        w.write_bool(*self);
    }
}

impl CdrEncode for i8 {
    #[inline]
    fn cdr_encode<BO: ByteOrder, B: CdrBuffer>(&self, w: &mut CdrWriter<'_, BO, B>) {
        w.write_i8(*self);
    }
}

impl CdrEncode for u8 {
    #[inline]
    fn cdr_encode<BO: ByteOrder, B: CdrBuffer>(&self, w: &mut CdrWriter<'_, BO, B>) {
        w.write_u8(*self);
    }
}

impl CdrEncode for i16 {
    #[inline]
    fn cdr_encode<BO: ByteOrder, B: CdrBuffer>(&self, w: &mut CdrWriter<'_, BO, B>) {
        w.write_i16(*self);
    }
}

impl CdrEncode for u16 {
    #[inline]
    fn cdr_encode<BO: ByteOrder, B: CdrBuffer>(&self, w: &mut CdrWriter<'_, BO, B>) {
        w.write_u16(*self);
    }
}

impl CdrEncode for i32 {
    #[inline]
    fn cdr_encode<BO: ByteOrder, B: CdrBuffer>(&self, w: &mut CdrWriter<'_, BO, B>) {
        w.write_i32(*self);
    }
}

impl CdrEncode for u32 {
    #[inline]
    fn cdr_encode<BO: ByteOrder, B: CdrBuffer>(&self, w: &mut CdrWriter<'_, BO, B>) {
        w.write_u32(*self);
    }
}

impl CdrEncode for i64 {
    #[inline]
    fn cdr_encode<BO: ByteOrder, B: CdrBuffer>(&self, w: &mut CdrWriter<'_, BO, B>) {
        w.write_i64(*self);
    }
}

impl CdrEncode for u64 {
    #[inline]
    fn cdr_encode<BO: ByteOrder, B: CdrBuffer>(&self, w: &mut CdrWriter<'_, BO, B>) {
        w.write_u64(*self);
    }
}

impl CdrEncode for f32 {
    #[inline]
    fn cdr_encode<BO: ByteOrder, B: CdrBuffer>(&self, w: &mut CdrWriter<'_, BO, B>) {
        w.write_f32(*self);
    }
}

impl CdrEncode for f64 {
    #[inline]
    fn cdr_encode<BO: ByteOrder, B: CdrBuffer>(&self, w: &mut CdrWriter<'_, BO, B>) {
        w.write_f64(*self);
    }
}

impl CdrEncode for String {
    #[inline]
    fn cdr_encode<BO: ByteOrder, B: CdrBuffer>(&self, w: &mut CdrWriter<'_, BO, B>) {
        w.write_string(self);
    }
}

impl CdrEncode for str {
    #[inline]
    fn cdr_encode<BO: ByteOrder, B: CdrBuffer>(&self, w: &mut CdrWriter<'_, BO, B>) {
        w.write_string(self);
    }
}

// Generic Vec<T> — element-by-element with length prefix.
// Note: Vec<u8> uses this path too (sequence of u8 with u32 length prefix).
// ZBuf is handled separately in the ZBuf CdrEncode impl in ros-z.
impl<T: CdrEncode> CdrEncode for Vec<T> {
    #[inline]
    fn cdr_encode<BO: ByteOrder, B: CdrBuffer>(&self, w: &mut CdrWriter<'_, BO, B>) {
        w.write_sequence_length(self.len());
        for item in self {
            item.cdr_encode(w);
        }
    }
}

impl<K: CdrEncode, V: CdrEncode> CdrEncode for BTreeMap<K, V> {
    #[inline]
    fn cdr_encode<BO: ByteOrder, B: CdrBuffer>(&self, w: &mut CdrWriter<'_, BO, B>) {
        w.write_sequence_length(self.len());
        for (key, value) in self {
            key.cdr_encode(w);
            value.cdr_encode(w);
        }
    }
}

impl<K: CdrEncode, V: CdrEncode> CdrEncode for HashMap<K, V> {
    #[inline]
    fn cdr_encode<BO: ByteOrder, B: CdrBuffer>(&self, w: &mut CdrWriter<'_, BO, B>) {
        w.write_sequence_length(self.len());
        for (key, value) in self {
            key.cdr_encode(w);
            value.cdr_encode(w);
        }
    }
}

// Fixed arrays — no length prefix, element-by-element.
impl<T: CdrEncode, const N: usize> CdrEncode for [T; N] {
    #[inline]
    fn cdr_encode<BO: ByteOrder, B: CdrBuffer>(&self, w: &mut CdrWriter<'_, BO, B>) {
        for item in self {
            item.cdr_encode(w);
        }
    }
}

// ── Primitive CdrDecode impls ───────────────────────────────────────────

impl CdrDecode for bool {
    #[inline]
    fn cdr_decode<'de, BO: ByteOrder>(r: &mut CdrReader<'de, BO>) -> Result<Self> {
        r.read_bool()
    }
}

impl CdrDecode for i8 {
    #[inline]
    fn cdr_decode<'de, BO: ByteOrder>(r: &mut CdrReader<'de, BO>) -> Result<Self> {
        r.read_i8()
    }
}

impl CdrDecode for u8 {
    #[inline]
    fn cdr_decode<'de, BO: ByteOrder>(r: &mut CdrReader<'de, BO>) -> Result<Self> {
        r.read_u8()
    }
}

impl CdrDecode for i16 {
    #[inline]
    fn cdr_decode<'de, BO: ByteOrder>(r: &mut CdrReader<'de, BO>) -> Result<Self> {
        r.read_i16()
    }
}

impl CdrDecode for u16 {
    #[inline]
    fn cdr_decode<'de, BO: ByteOrder>(r: &mut CdrReader<'de, BO>) -> Result<Self> {
        r.read_u16()
    }
}

impl CdrDecode for i32 {
    #[inline]
    fn cdr_decode<'de, BO: ByteOrder>(r: &mut CdrReader<'de, BO>) -> Result<Self> {
        r.read_i32()
    }
}

impl CdrDecode for u32 {
    #[inline]
    fn cdr_decode<'de, BO: ByteOrder>(r: &mut CdrReader<'de, BO>) -> Result<Self> {
        r.read_u32()
    }
}

impl CdrDecode for i64 {
    #[inline]
    fn cdr_decode<'de, BO: ByteOrder>(r: &mut CdrReader<'de, BO>) -> Result<Self> {
        r.read_i64()
    }
}

impl CdrDecode for u64 {
    #[inline]
    fn cdr_decode<'de, BO: ByteOrder>(r: &mut CdrReader<'de, BO>) -> Result<Self> {
        r.read_u64()
    }
}

impl CdrDecode for f32 {
    #[inline]
    fn cdr_decode<'de, BO: ByteOrder>(r: &mut CdrReader<'de, BO>) -> Result<Self> {
        r.read_f32()
    }
}

impl CdrDecode for f64 {
    #[inline]
    fn cdr_decode<'de, BO: ByteOrder>(r: &mut CdrReader<'de, BO>) -> Result<Self> {
        r.read_f64()
    }
}

impl CdrDecode for String {
    #[inline]
    fn cdr_decode<'de, BO: ByteOrder>(r: &mut CdrReader<'de, BO>) -> Result<Self> {
        r.read_string()
    }
}

// Generic Vec<T> — element-by-element with length prefix.
impl<T: CdrDecode> CdrDecode for Vec<T> {
    #[inline]
    fn cdr_decode<'de, BO: ByteOrder>(r: &mut CdrReader<'de, BO>) -> Result<Self> {
        let count = r.read_sequence_length()?;
        let mut out = Vec::with_capacity(count);
        for _ in 0..count {
            out.push(T::cdr_decode(r)?);
        }
        Ok(out)
    }
}

impl<K: CdrDecode + Ord, V: CdrDecode> CdrDecode for BTreeMap<K, V> {
    #[inline]
    fn cdr_decode<'de, BO: ByteOrder>(r: &mut CdrReader<'de, BO>) -> Result<Self> {
        let count = r.read_sequence_length()?;
        let mut out = BTreeMap::new();
        for _ in 0..count {
            out.insert(K::cdr_decode(r)?, V::cdr_decode(r)?);
        }
        Ok(out)
    }
}

impl<K: CdrDecode + Eq + Hash, V: CdrDecode> CdrDecode for HashMap<K, V> {
    #[inline]
    fn cdr_decode<'de, BO: ByteOrder>(r: &mut CdrReader<'de, BO>) -> Result<Self> {
        let count = r.read_sequence_length()?;
        let mut out = HashMap::with_capacity(count);
        for _ in 0..count {
            out.insert(K::cdr_decode(r)?, V::cdr_decode(r)?);
        }
        Ok(out)
    }
}

// Fixed arrays — no length prefix, element-by-element.
impl<T: CdrDecode + Default, const N: usize> CdrDecode for [T; N] {
    fn cdr_decode<'de, BO: ByteOrder>(r: &mut CdrReader<'de, BO>) -> Result<Self> {
        // Can't use array::try_from_fn on stable yet, so build via Vec.
        let mut v: Vec<T> = Vec::with_capacity(N);
        for _ in 0..N {
            v.push(T::cdr_decode(r)?);
        }
        // SAFETY: we just pushed exactly N elements.
        let arr: [T; N] = v.try_into().unwrap_or_else(|_| unreachable!());
        Ok(arr)
    }
}

// ── Primitive CdrEncodedSize impls ────────────────────────────────────────

/// Alignment helper: bytes needed to align `pos` to `align`.
#[inline]
fn padding(pos: usize, align: usize) -> usize {
    let modulo = pos % align;
    if modulo == 0 { 0 } else { align - modulo }
}

impl CdrEncodedSize for bool {
    #[inline]
    fn cdr_encoded_size(&self, pos: usize) -> usize {
        pos + 1
    }
}

impl CdrEncodedSize for i8 {
    #[inline]
    fn cdr_encoded_size(&self, pos: usize) -> usize {
        pos + 1
    }
}

impl CdrEncodedSize for u8 {
    #[inline]
    fn cdr_encoded_size(&self, pos: usize) -> usize {
        pos + 1
    }
}

impl CdrEncodedSize for i16 {
    #[inline]
    fn cdr_encoded_size(&self, pos: usize) -> usize {
        pos + padding(pos, 2) + 2
    }
}

impl CdrEncodedSize for u16 {
    #[inline]
    fn cdr_encoded_size(&self, pos: usize) -> usize {
        pos + padding(pos, 2) + 2
    }
}

impl CdrEncodedSize for i32 {
    #[inline]
    fn cdr_encoded_size(&self, pos: usize) -> usize {
        pos + padding(pos, 4) + 4
    }
}

impl CdrEncodedSize for u32 {
    #[inline]
    fn cdr_encoded_size(&self, pos: usize) -> usize {
        pos + padding(pos, 4) + 4
    }
}

impl CdrEncodedSize for i64 {
    #[inline]
    fn cdr_encoded_size(&self, pos: usize) -> usize {
        pos + padding(pos, 8) + 8
    }
}

impl CdrEncodedSize for u64 {
    #[inline]
    fn cdr_encoded_size(&self, pos: usize) -> usize {
        pos + padding(pos, 8) + 8
    }
}

impl CdrEncodedSize for f32 {
    #[inline]
    fn cdr_encoded_size(&self, pos: usize) -> usize {
        pos + padding(pos, 4) + 4
    }
}

impl CdrEncodedSize for f64 {
    #[inline]
    fn cdr_encoded_size(&self, pos: usize) -> usize {
        pos + padding(pos, 8) + 8
    }
}

impl CdrEncodedSize for String {
    #[inline]
    fn cdr_encoded_size(&self, pos: usize) -> usize {
        // u32 length prefix (4-byte aligned) + string bytes + null terminator
        let after_len = pos + padding(pos, 4) + 4;
        after_len + self.len() + 1
    }
}

impl<T: CdrEncodedSize> CdrEncodedSize for Vec<T> {
    #[inline]
    fn cdr_encoded_size(&self, pos: usize) -> usize {
        let mut p = pos + padding(pos, 4) + 4; // sequence length u32
        for item in self {
            p = item.cdr_encoded_size(p);
        }
        p
    }
}

impl<K: CdrEncodedSize, V: CdrEncodedSize> CdrEncodedSize for BTreeMap<K, V> {
    #[inline]
    fn cdr_encoded_size(&self, pos: usize) -> usize {
        let mut p = pos + padding(pos, 4) + 4;
        for (key, value) in self {
            p = key.cdr_encoded_size(p);
            p = value.cdr_encoded_size(p);
        }
        p
    }
}

impl<K: CdrEncodedSize, V: CdrEncodedSize> CdrEncodedSize for HashMap<K, V> {
    #[inline]
    fn cdr_encoded_size(&self, pos: usize) -> usize {
        let mut p = pos + padding(pos, 4) + 4;
        for (key, value) in self {
            p = key.cdr_encoded_size(p);
            p = value.cdr_encoded_size(p);
        }
        p
    }
}

impl<T: CdrEncodedSize, const N: usize> CdrEncodedSize for [T; N] {
    #[inline]
    fn cdr_encoded_size(&self, pos: usize) -> usize {
        let mut p = pos;
        for item in self {
            p = item.cdr_encoded_size(p);
        }
        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use byteorder::LittleEndian;

    fn roundtrip<T: CdrEncode + CdrDecode + std::fmt::Debug + PartialEq>(value: &T) -> T {
        let buf = cdr_to_vec(value, 64);
        let mut reader = CdrReader::<LittleEndian>::new(&buf);
        T::cdr_decode(&mut reader).expect("deserialize failed")
    }

    #[test]
    #[allow(clippy::approx_constant, clippy::bool_assert_comparison)]
    fn test_primitives_roundtrip() {
        assert_eq!(roundtrip(&42i32), 42i32);
        assert_eq!(roundtrip(&3.14f64), 3.14f64);
        assert_eq!(roundtrip(&true), true);
        assert_eq!(roundtrip(&"hello".to_string()), "hello".to_string());
    }

    #[test]
    fn test_vec_roundtrip() {
        let v: Vec<i32> = vec![1, 2, 3, 4, 5];
        assert_eq!(roundtrip(&v), v);
    }

    #[test]
    fn test_vec_u8_roundtrip() {
        // Vec<u8> uses the generic Vec<T> path (u32 count + elements)
        let v: Vec<u8> = vec![10, 20, 30];
        assert_eq!(roundtrip(&v), v);
    }

    #[test]
    fn test_btreemap_roundtrip() {
        let mut map = BTreeMap::new();
        map.insert("ball".to_string(), 3u32);
        map.insert("robot".to_string(), 7u32);
        assert_eq!(roundtrip(&map), map);
    }

    #[test]
    fn test_hashmap_roundtrip() {
        let mut map = HashMap::new();
        map.insert("ball".to_string(), 3u32);
        map.insert("robot".to_string(), 7u32);
        assert_eq!(roundtrip(&map), map);
    }

    #[test]
    fn test_string_roundtrip() {
        let s = "hello, world".to_string();
        assert_eq!(roundtrip(&s), s);
    }
}
