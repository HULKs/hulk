//! Low-level CDR primitive read/write operations.
//!
//! This module provides `CdrWriter` and `CdrReader` for direct CDR byte
//! manipulation with proper alignment handling. These are used internally
//! by the serde-based serializer/deserializer and can also be used directly
//! for schema-driven (dynamic) message handling.

#[cfg(target_endian = "little")]
use bytemuck;
use byteorder::ByteOrder;
use std::borrow::Cow;
use std::marker::PhantomData;
use zenoh_buffers::{ZBuf, ZSlice};

use crate::buffer::CdrBuffer;
use crate::error::{Error, Result};
use crate::plain::CdrPlain;

/// Low-level CDR writer with alignment handling.
///
/// Provides primitive write operations that handle CDR alignment requirements.
/// Used internally by `SerdeCdrSerializer` and available for direct use with
/// dynamic/schema-driven message serialization.
pub struct CdrWriter<'a, BO, B: CdrBuffer = Vec<u8>> {
    buffer: &'a mut B,
    start_offset: usize,
    _phantom: PhantomData<BO>,
}

impl<'a, BO: ByteOrder, B: CdrBuffer> CdrWriter<'a, BO, B> {
    /// Create a new writer for the given buffer.
    #[inline]
    pub fn new(buffer: &'a mut B) -> Self {
        let start_offset = buffer.len();
        Self {
            buffer,
            start_offset,
            _phantom: PhantomData,
        }
    }

    /// Current position relative to start.
    #[inline(always)]
    pub fn position(&self) -> usize {
        self.buffer.len() - self.start_offset
    }

    /// Add padding bytes for alignment.
    #[inline(always)]
    pub fn align(&mut self, alignment: usize) {
        let modulo = self.position() % alignment;
        if modulo != 0 {
            let padding = alignment - modulo;
            self.buffer.reserve(padding);
            for _ in 0..padding {
                self.buffer.push(0);
            }
        }
    }

    /// Get mutable access to the underlying buffer.
    #[inline]
    pub fn buffer_mut(&mut self) -> &mut B {
        self.buffer
    }

    /// Reborrow without changing the alignment origin (for field-level adapters).
    pub fn reborrow(&mut self) -> CdrWriter<'_, BO, B> {
        CdrWriter {
            buffer: self.buffer,
            start_offset: self.start_offset,
            _phantom: PhantomData,
        }
    }

    // Primitive write operations

    #[inline]
    pub fn write_bool(&mut self, v: bool) {
        self.buffer.push(if v { 1 } else { 0 });
    }

    #[inline]
    pub fn write_i8(&mut self, v: i8) {
        self.buffer.push(v as u8);
    }

    #[inline]
    pub fn write_u8(&mut self, v: u8) {
        self.buffer.push(v);
    }

    #[inline]
    pub fn write_i16(&mut self, v: i16) {
        self.align(2);
        let mut buf = [0u8; 2];
        BO::write_i16(&mut buf, v);
        self.buffer.extend_from_slice(&buf);
    }

    #[inline]
    pub fn write_u16(&mut self, v: u16) {
        self.align(2);
        let mut buf = [0u8; 2];
        BO::write_u16(&mut buf, v);
        self.buffer.extend_from_slice(&buf);
    }

    #[inline]
    pub fn write_i32(&mut self, v: i32) {
        self.align(4);
        let mut buf = [0u8; 4];
        BO::write_i32(&mut buf, v);
        self.buffer.extend_from_slice(&buf);
    }

    #[inline]
    pub fn write_u32(&mut self, v: u32) {
        self.align(4);
        let mut buf = [0u8; 4];
        BO::write_u32(&mut buf, v);
        self.buffer.extend_from_slice(&buf);
    }

    #[inline]
    pub fn write_i64(&mut self, v: i64) {
        self.align(8);
        let mut buf = [0u8; 8];
        BO::write_i64(&mut buf, v);
        self.buffer.extend_from_slice(&buf);
    }

    #[inline]
    pub fn write_u64(&mut self, v: u64) {
        self.align(8);
        let mut buf = [0u8; 8];
        BO::write_u64(&mut buf, v);
        self.buffer.extend_from_slice(&buf);
    }

    #[inline]
    pub fn write_f32(&mut self, v: f32) {
        self.align(4);
        let mut buf = [0u8; 4];
        BO::write_f32(&mut buf, v);
        self.buffer.extend_from_slice(&buf);
    }

    #[inline]
    pub fn write_f64(&mut self, v: f64) {
        self.align(8);
        let mut buf = [0u8; 8];
        BO::write_f64(&mut buf, v);
        self.buffer.extend_from_slice(&buf);
    }

    /// Write a CDR string (length-prefixed with null terminator).
    #[inline]
    pub fn write_string(&mut self, s: &str) {
        let byte_count = s.len() as u32 + 1; // Include null terminator
        self.write_u32(byte_count);
        self.buffer.extend_from_slice(s.as_bytes());
        self.buffer.push(0); // Null terminator
    }

    /// Write raw bytes with length prefix (for sequences of u8).
    #[inline]
    pub fn write_bytes(&mut self, bytes: &[u8]) {
        self.write_u32(bytes.len() as u32);
        self.buffer.extend_from_slice(bytes);
    }

    /// Write a sequence length prefix.
    #[inline]
    pub fn write_sequence_length(&mut self, len: usize) {
        self.write_u32(len as u32);
    }

    /// Bulk-write a slice of plain (POD) values as raw bytes.
    ///
    /// The caller must write the sequence length prefix separately before calling this.
    /// Alignment is handled internally based on `T`'s alignment requirement.
    ///
    /// Only available on little-endian hosts where CDR wire layout == memory layout.
    #[cfg(target_endian = "little")]
    #[inline]
    pub fn write_pod_slice<T: CdrPlain + bytemuck::Pod>(&mut self, slice: &[T]) {
        debug_assert!(!slice.is_empty());
        self.align(T::CDR_ALIGNMENT);
        self.buffer.extend_from_slice(bytemuck::cast_slice(slice));
    }
}

/// Low-level CDR reader with alignment handling.
///
/// Provides primitive read operations that handle CDR alignment requirements.
/// Used internally by `CdrDeserializer` and available for direct use with
/// dynamic/schema-driven message deserialization.
pub struct CdrReader<'a, BO> {
    backing: ReaderBacking<'a>,
    length: usize,
    position: usize,
    _phantom: PhantomData<BO>,
}

enum ReaderBacking<'a> {
    Borrowed {
        input: &'a [u8],
        offset: usize,
    },
    Owned {
        slice: ZSlice,
        offset: usize,
    },
    Segmented {
        slices: Vec<ZSlice>,
        index: usize,
        offset: usize,
    },
}

impl<'a, BO: ByteOrder> CdrReader<'a, BO> {
    /// Create a new reader for the given input bytes.
    #[inline]
    pub fn new(input: &'a [u8]) -> Self {
        Self {
            backing: ReaderBacking::Borrowed { input, offset: 0 },
            length: input.len(),
            position: 0,
            _phantom: PhantomData,
        }
    }

    /// Retain payload owners; individual contiguous byte fields can be shared.
    pub fn from_zbuf(input: &ZBuf) -> Result<Self> {
        let mut slices = input.zslices().filter(|s| !s.is_empty()).peekable();
        let Some(first) = slices.next() else {
            return Ok(Self::new(&[]));
        };
        if slices.peek().is_none() {
            return Ok(Self {
                length: first.len(),
                position: 0,
                backing: ReaderBacking::Owned {
                    slice: first.clone(),
                    offset: 0,
                },
                _phantom: PhantomData,
            });
        }
        let slices: Vec<_> = std::iter::once(first).chain(slices).cloned().collect();
        let length = slices.iter().try_fold(0usize, |length, slice| {
            length
                .checked_add(slice.len())
                .ok_or(Error::LengthOverflow {
                    kind: "payload",
                    len: usize::MAX,
                })
        })?;
        Ok(Self {
            backing: ReaderBacking::Segmented {
                slices,
                index: 0,
                offset: 0,
            },
            length,
            position: 0,
            _phantom: PhantomData,
        })
    }

    /// Reset alignment after the encapsulation header, retaining the payload owners.
    pub fn reset_origin(&mut self) {
        self.length -= self.position;
        self.position = 0;
    }

    // Call only after checking remaining(); backing cursors are absolute, position is logical.
    fn advance(&mut self, mut count: usize) {
        self.position += count;
        match &mut self.backing {
            ReaderBacking::Borrowed { offset, .. } | ReaderBacking::Owned { offset, .. } => {
                *offset += count
            }
            ReaderBacking::Segmented {
                slices,
                index,
                offset,
            } => {
                while count > 0 {
                    let available = slices[*index].len() - *offset;
                    let consumed = count.min(available);
                    *offset += consumed;
                    count -= consumed;
                    if *offset == slices[*index].len() {
                        *index += 1;
                        *offset = 0;
                    }
                }
            }
        }
    }

    /// Read one owned field. Only fields spanning segments need a bulk copy.
    pub fn read_zslice(&mut self, count: usize) -> Result<ZSlice> {
        if count > self.remaining() {
            return Err(Error::UnexpectedEof);
        }
        if count == 0 {
            return Ok(ZSlice::from(Vec::new()));
        }
        let contiguous = match &self.backing {
            ReaderBacking::Owned { slice, offset } => Some((slice, *offset)),
            ReaderBacking::Segmented {
                slices,
                index,
                offset,
            } => slices.get(*index).map(|slice| (slice, *offset)),
            ReaderBacking::Borrowed { .. } => None,
        };
        if let Some((slice, offset)) = contiguous
            && count <= slice.len() - offset
        {
            let result = slice
                .subslice(offset..offset + count)
                .ok_or(Error::UnexpectedEof)?;
            self.advance(count);
            return Ok(result);
        }
        Ok(ZSlice::from(self.read_bytes(count)?.into_owned()))
    }

    /// Current read position.
    #[inline]
    pub fn position(&self) -> usize {
        self.position
    }

    /// Remaining bytes available.
    #[inline]
    pub fn remaining(&self) -> usize {
        self.length - self.position
    }

    /// Align to the given boundary.
    #[inline]
    pub fn align(&mut self, alignment: usize) -> Result<()> {
        let modulo = self.position % alignment;
        if modulo != 0 {
            let padding = alignment - modulo;
            if self.remaining() < padding {
                return Err(Error::UnexpectedEof);
            }
            self.advance(padding);
        }
        Ok(())
    }

    /// Read raw bytes without alignment.
    #[inline]
    pub fn read_bytes(&mut self, count: usize) -> Result<Cow<'a, [u8]>> {
        if self.remaining() < count {
            return Err(Error::UnexpectedEof);
        }
        if let ReaderBacking::Borrowed { input, offset } = &self.backing {
            let bytes = &input[*offset..*offset + count];
            self.advance(count);
            return Ok(Cow::Borrowed(bytes));
        }
        let mut bytes = vec![0; count];
        self.read_into(&mut bytes)?;
        Ok(Cow::Owned(bytes))
    }

    /// Copy a primitive or fragmented field without allocating scratch storage.
    pub fn read_into(&mut self, output: &mut [u8]) -> Result<()> {
        if output.len() > self.remaining() {
            return Err(Error::UnexpectedEof);
        }
        match &self.backing {
            ReaderBacking::Borrowed { input, offset } => {
                output.copy_from_slice(&input[*offset..*offset + output.len()]);
            }
            ReaderBacking::Owned { slice, offset } => {
                output.copy_from_slice(&slice[*offset..*offset + output.len()]);
            }
            ReaderBacking::Segmented {
                slices,
                index,
                offset,
            } => {
                let mut skip = *offset;
                let mut written = 0;
                for slice in &slices[*index..] {
                    let offset = skip.min(slice.len());
                    skip -= offset;
                    let count = (output.len() - written).min(slice.len() - offset);
                    output[written..written + count]
                        .copy_from_slice(&slice.as_slice()[offset..offset + count]);
                    written += count;
                    if written == output.len() {
                        break;
                    }
                }
            }
        }
        self.advance(output.len());
        Ok(())
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N]> {
        let mut bytes = [0; N];
        self.read_into(&mut bytes)?;
        Ok(bytes)
    }

    // Primitive read operations

    #[inline]
    pub fn read_bool(&mut self) -> Result<bool> {
        let byte = self.read_u8()?;
        match byte {
            0 => Ok(false),
            1 => Ok(true),
            x => Err(Error::InvalidBool(x)),
        }
    }

    #[inline]
    pub fn read_i8(&mut self) -> Result<i8> {
        Ok(self.read_u8()? as i8)
    }

    #[inline]
    pub fn read_u8(&mut self) -> Result<u8> {
        Ok(self.read_array::<1>()?[0])
    }

    #[inline]
    pub fn read_i16(&mut self) -> Result<i16> {
        self.align(2)?;
        Ok(BO::read_i16(&self.read_array::<2>()?))
    }

    #[inline]
    pub fn read_u16(&mut self) -> Result<u16> {
        self.align(2)?;
        Ok(BO::read_u16(&self.read_array::<2>()?))
    }

    #[inline]
    pub fn read_i32(&mut self) -> Result<i32> {
        self.align(4)?;
        Ok(BO::read_i32(&self.read_array::<4>()?))
    }

    #[inline]
    pub fn read_u32(&mut self) -> Result<u32> {
        self.align(4)?;
        Ok(BO::read_u32(&self.read_array::<4>()?))
    }

    #[inline]
    pub fn read_i64(&mut self) -> Result<i64> {
        self.align(8)?;
        Ok(BO::read_i64(&self.read_array::<8>()?))
    }

    #[inline]
    pub fn read_u64(&mut self) -> Result<u64> {
        self.align(8)?;
        Ok(BO::read_u64(&self.read_array::<8>()?))
    }

    #[inline]
    pub fn read_f32(&mut self) -> Result<f32> {
        self.align(4)?;
        Ok(BO::read_f32(&self.read_array::<4>()?))
    }

    #[inline]
    pub fn read_f64(&mut self) -> Result<f64> {
        self.align(8)?;
        Ok(BO::read_f64(&self.read_array::<8>()?))
    }

    /// Read a CDR string (length-prefixed with null terminator).
    #[inline]
    pub fn read_string(&mut self) -> Result<String> {
        Ok(self.read_str()?.into_owned())
    }

    /// Read a CDR string as a borrowed slice (zero-copy when possible).
    #[inline]
    pub fn read_str(&mut self) -> Result<Cow<'a, str>> {
        let len = self.read_u32()? as usize;
        let bytes = self.read_bytes(len)?;
        match bytes {
            Cow::Borrowed(bytes) => {
                let bytes = bytes
                    .strip_suffix(&[0])
                    .ok_or(Error::InvalidStringTerminator)?;
                Ok(Cow::Borrowed(
                    std::str::from_utf8(bytes).map_err(Error::Utf8)?,
                ))
            }
            Cow::Owned(mut bytes) => {
                if bytes.pop() != Some(0) {
                    return Err(Error::InvalidStringTerminator);
                }
                Ok(Cow::Owned(
                    String::from_utf8(bytes).map_err(|e| Error::Utf8(e.utf8_error()))?,
                ))
            }
        }
    }

    /// Read a sequence length prefix.
    ///
    /// # Safety
    /// Includes a sanity check to prevent absurd allocations from malformed data.
    /// Maximum sequence length is 100 million elements (~400MB for float32 arrays).
    #[inline]
    pub fn read_sequence_length(&mut self) -> Result<usize> {
        const MAX_SEQUENCE_LENGTH: u32 = 100_000_000;
        let len = self.read_u32()?;
        if len > MAX_SEQUENCE_LENGTH {
            return Err(Error::Custom(format!(
                "Sequence length {} exceeds maximum allowed ({}). Possible schema mismatch or corrupted data.",
                len, MAX_SEQUENCE_LENGTH
            )));
        }
        Ok(len as usize)
    }

    /// Read raw bytes with length prefix.
    #[inline]
    pub fn read_byte_sequence(&mut self) -> Result<Cow<'a, [u8]>> {
        let len = self.read_u32()? as usize;
        self.read_bytes(len)
    }

    /// Bulk-read `count` plain (POD) values into an aligned owned allocation.
    ///
    /// The caller must have already read the sequence length prefix.
    /// Alignment is handled internally based on `T`'s alignment requirement.
    ///
    /// Only available on little-endian hosts where CDR wire layout == memory layout.
    #[cfg(target_endian = "little")]
    #[inline]
    pub fn read_pod_slice<T: CdrPlain + bytemuck::Pod>(&mut self, count: usize) -> Result<Vec<T>> {
        if count == 0 {
            return Ok(vec![]);
        }
        self.align(T::CDR_ALIGNMENT)?;
        let byte_count = count
            .checked_mul(std::mem::size_of::<T>())
            .ok_or(Error::UnexpectedEof)?;
        if byte_count > self.remaining() {
            return Err(Error::UnexpectedEof);
        }
        let mut values = vec![T::zeroed(); count];
        self.read_into(bytemuck::cast_slice_mut(&mut values))?;
        Ok(values)
    }

    #[cfg(target_endian = "little")]
    /// Bulk-read a fixed array without a temporary allocation.
    pub fn read_pod_array<T: CdrPlain + bytemuck::Pod, const N: usize>(
        &mut self,
    ) -> Result<[T; N]> {
        let mut values = [T::zeroed(); N];
        if N != 0 {
            self.align(T::CDR_ALIGNMENT)?;
            self.read_into(bytemuck::cast_slice_mut(&mut values))?;
        }
        Ok(values)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use byteorder::LittleEndian;

    #[test]
    fn empty_segmented_reader_and_zero_length_reads() {
        let mut buffer = ZBuf::empty();
        buffer.push_zslice(Vec::<u8>::new().into());
        let mut reader = CdrReader::<LittleEndian>::from_zbuf(&buffer).unwrap();
        assert_eq!(reader.remaining(), 0);
        assert!(matches!(reader.backing, ReaderBacking::Borrowed { .. }));
        assert!(reader.read_bytes(0).unwrap().is_empty());
        assert!(reader.read_zslice(0).unwrap().is_empty());
        reader.reset_origin();
        assert!(reader.read_u8().is_err());
        assert_eq!(reader.position(), 0);
    }

    #[test]
    fn reader_backings_preserve_alignment_and_bridge_cursor() {
        use serde::Deserialize;
        let mut bytes = vec![0, 1, 0, 0];
        let mut writer = CdrWriter::<LittleEndian>::new(&mut bytes);
        writer.write_u8(3);
        writer.write_u64(17);
        writer.write_bytes(&[5, 6, 7]);
        writer.write_u32(99);

        fn check(mut reader: CdrReader<'_, LittleEndian>) {
            assert_eq!(reader.read_bytes(4).unwrap().as_ref(), &[0, 1, 0, 0]);
            reader.reset_origin();
            assert_eq!(reader.read_u8().unwrap(), 3);
            let mut deserializer = crate::CdrDeserializer::from_reader(reader);
            assert_eq!(u64::deserialize(&mut deserializer).unwrap(), 17);
            let mut reader = deserializer.into_reader();
            let count = reader.read_sequence_length().unwrap();
            assert_eq!(reader.read_zslice(count).unwrap().as_slice(), &[5, 6, 7]);
            assert_eq!(reader.read_u32().unwrap(), 99);
            assert_eq!(reader.remaining(), 0);
            reader.reset_origin();
            assert_eq!(reader.position(), 0);
            assert!(reader.read_u8().is_err());
        }
        check(CdrReader::new(&bytes));
        for split in 0..=bytes.len() {
            let mut buffer = ZBuf::empty();
            buffer.push_zslice(bytes[..split].to_vec().into());
            buffer.push_zslice(Vec::<u8>::new().into());
            buffer.push_zslice(bytes[split..].to_vec().into());
            check(CdrReader::from_zbuf(&buffer).unwrap());
        }
    }

    #[test]
    fn reset_origin_keeps_segment_owner_and_absolute_cursor() {
        let slice = ZSlice::from(vec![1, 2, 3, 4, 5, 6]);
        let pointer = slice.as_ptr();
        let buffer = ZBuf::from(slice);
        let mut reader = CdrReader::<LittleEndian>::from_zbuf(&buffer).unwrap();
        assert!(matches!(reader.backing, ReaderBacking::Owned { .. }));
        drop(buffer);
        reader.read_bytes(2).unwrap();
        reader.reset_origin();
        reader.read_bytes(1).unwrap();
        reader.reset_origin();
        let tail = reader.read_zslice(3).unwrap();
        assert_eq!(tail.as_ptr(), pointer.wrapping_add(3));
        drop(reader);
        assert_eq!(tail.as_slice(), &[4, 5, 6]);
    }

    #[test]
    fn test_reader_primitives() {
        let mut buffer = Vec::new();
        {
            let mut writer = CdrWriter::<LittleEndian>::new(&mut buffer);
            writer.write_bool(true);
            writer.write_u8(42);
            writer.write_u32(12345);
            writer.write_f64(1.23456);
            writer.write_string("hello");
        }

        let mut reader = CdrReader::<LittleEndian>::new(&buffer);
        assert!(reader.read_bool().unwrap());
        assert_eq!(reader.read_u8().unwrap(), 42);
        assert_eq!(reader.read_u32().unwrap(), 12345);
        assert!((reader.read_f64().unwrap() - 1.23456).abs() < 0.00001);
        assert_eq!(reader.read_string().unwrap(), "hello");
    }

    #[test]
    fn test_alignment() {
        let mut buffer = Vec::new();
        let mut writer = CdrWriter::<LittleEndian>::new(&mut buffer);

        writer.write_u8(1); // position 1
        writer.write_u32(100); // should align to 4, so padding at 1,2,3

        // Buffer should be: [1, 0, 0, 0, 100, 0, 0, 0]
        assert_eq!(buffer.len(), 8);
        assert_eq!(buffer[0], 1);
        assert_eq!(buffer[1], 0); // padding
        assert_eq!(buffer[2], 0); // padding
        assert_eq!(buffer[3], 0); // padding
    }

    #[test]
    fn test_roundtrip_all_types() {
        let mut buffer = Vec::new();
        {
            let mut writer = CdrWriter::<LittleEndian>::new(&mut buffer);
            writer.write_bool(false);
            writer.write_i8(-42);
            writer.write_i16(-1000);
            writer.write_i32(-100000);
            writer.write_i64(-10000000000);
            writer.write_u8(200);
            writer.write_u16(50000);
            writer.write_u32(3000000000);
            writer.write_u64(10000000000);
            writer.write_f32(1.5);
            writer.write_f64(9.87654321);
            writer.write_string("test string");
        }

        let mut reader = CdrReader::<LittleEndian>::new(&buffer);
        assert!(!reader.read_bool().unwrap());
        assert_eq!(reader.read_i8().unwrap(), -42);
        assert_eq!(reader.read_i16().unwrap(), -1000);
        assert_eq!(reader.read_i32().unwrap(), -100000);
        assert_eq!(reader.read_i64().unwrap(), -10000000000);
        assert_eq!(reader.read_u8().unwrap(), 200);
        assert_eq!(reader.read_u16().unwrap(), 50000);
        assert_eq!(reader.read_u32().unwrap(), 3000000000);
        assert_eq!(reader.read_u64().unwrap(), 10000000000);
        assert!((reader.read_f32().unwrap() - 1.5).abs() < 0.001);
        assert!((reader.read_f64().unwrap() - 9.87654321).abs() < 0.0000001);
        assert_eq!(reader.read_string().unwrap(), "test string");
    }
}
