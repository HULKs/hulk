//! Composable native CDR fields. Foreign types keep checked, alignment-preserving
//! Serde bridges through internal field helpers.
use crate::message::{CDR_HEADER_LE, CdrEncodeError, CdrError, Message, WireDecoder, WireEncoder};
use byteorder::LittleEndian;
use ros_z_cdr::{CdrBuffer, CdrReader, CdrWriter, ZBufWriter};
use std::marker::PhantomData;
use zenoh_buffers::{ZBuf, ZSlice};

/// Native message codec; nested fields share one CDR alignment origin.
pub struct CdrCodec<T>(PhantomData<T>);

// Count one field traversal without allocating or copying payload segments.
#[derive(Default)]
pub(crate) struct SizeCounter {
    size: usize,
    overflow: bool,
}

impl SizeCounter {
    pub(crate) fn finish(self) -> ros_z_cdr::Result<usize> {
        if self.overflow {
            Err(ros_z_cdr::Error::LengthOverflow {
                kind: "payload",
                len: usize::MAX,
            })
        } else {
            Ok(self.size)
        }
    }
    fn add(&mut self, count: usize) {
        match self.size.checked_add(count) {
            Some(size) => self.size = size,
            None => self.overflow = true,
        }
    }
}

impl CdrBuffer for SizeCounter {
    fn extend_from_slice(&mut self, data: &[u8]) {
        self.add(data.len());
    }
    fn push(&mut self, _: u8) {
        self.add(1);
    }
    fn len(&self) -> usize {
        self.size
    }
    fn clear(&mut self) {
        *self = Self::default();
    }
    fn append_zbuf(&mut self, buffer: &ZBuf) {
        use zenoh_buffers::buffer::Buffer;
        self.add(buffer.len());
    }
}

pub(crate) fn write_len<B: CdrBuffer>(
    writer: &mut CdrWriter<'_, LittleEndian, B>,
    len: usize,
) -> ros_z_cdr::Result<()> {
    writer.write_u32(
        u32::try_from(len).map_err(|_| ros_z_cdr::Error::LengthOverflow {
            kind: "sequence",
            len,
        })?,
    );
    Ok(())
}

impl<T: Message> WireEncoder for CdrCodec<T> {
    type Input<'a> = &'a T;
    type Error = CdrEncodeError;
    fn serialize_to_zbuf(input: &T) -> Result<ZBuf, Self::Error> {
        Self::serialize_to_zbuf_with_hint(input, 256)
    }
    fn serialize_to_zbuf_with_hint(input: &T, capacity: usize) -> Result<ZBuf, Self::Error> {
        let mut buffer = ZBufWriter::with_capacity(capacity);
        buffer.extend_from_slice(&CDR_HEADER_LE);
        input.encode_cdr(&mut CdrWriter::new(&mut buffer))?;
        Ok(buffer.into_zbuf())
    }
    fn serialized_size_hint(input: &T) -> usize {
        // Exact for deterministic Message impls; invalid values fail during encoding.
        Self::serialized_size(input).unwrap_or(0)
    }
    fn serialize_to_buf(input: &T, buffer: &mut Vec<u8>) -> Result<(), Self::Error> {
        buffer.clear();
        buffer.extend_from_slice(&CDR_HEADER_LE);
        input.encode_cdr(&mut CdrWriter::new(buffer))?;
        Ok(())
    }
    fn serialize_to_shm(
        input: &T,
        estimated_size: usize,
        provider: &zenoh::shm::ShmProvider<zenoh::shm::PosixShmProviderBackend>,
    ) -> crate::Result<(ZBuf, usize)> {
        let mut buffer =
            crate::shm::ShmWriter::new(provider, estimated_size.max(4)).or_else(|error| {
                let size = Self::serialized_size(input)
                    .map_err(|e| crate::Error::encode(std::any::type_name::<T>(), e))?;
                if size >= estimated_size {
                    return Err(error);
                }
                crate::shm::ShmWriter::new(provider, size)
            })?;
        buffer.extend_from_slice(&CDR_HEADER_LE);
        input
            .encode_cdr(&mut CdrWriter::new(&mut buffer))
            .map_err(|e| {
                crate::Error::encode(std::any::type_name::<T>(), CdrEncodeError::from(e))
            })?;
        if buffer.overflowed() {
            // The writer tracks the full length even when a caller's hint is too low.
            let size = buffer.position();
            drop(buffer);
            buffer = crate::shm::ShmWriter::new(provider, size)?;
            buffer.extend_from_slice(&CDR_HEADER_LE);
            input
                .encode_cdr(&mut CdrWriter::new(&mut buffer))
                .map_err(|e| crate::Error::encode(std::any::type_name::<T>(), e))?;
        }
        let actual_size = buffer.position();
        Ok((buffer.into_zbuf()?, actual_size))
    }
}

impl<T: Message> CdrCodec<T> {
    fn serialized_size(input: &T) -> Result<usize, CdrEncodeError> {
        let mut buffer = SizeCounter::default();
        buffer.extend_from_slice(&CDR_HEADER_LE);
        input.encode_cdr(&mut CdrWriter::new(&mut buffer))?;
        Ok(buffer.finish()?)
    }
    fn decode(mut reader: CdrReader<'_, LittleEndian>) -> Result<T, CdrError> {
        let mut header = [0; 4];
        reader.read_into(&mut header)?;
        if header[..2] != CDR_HEADER_LE[..2] {
            return Err(ros_z_cdr::Error::Custom("expected CDR_LE encapsulation".into()).into());
        }
        reader.reset_origin();
        Ok(T::decode_cdr(&mut reader)?)
    }
}

impl<T: Message> WireDecoder for CdrCodec<T> {
    type Input<'a> = &'a [u8];
    type Output = T;
    type Error = CdrError;
    fn deserialize(input: &[u8]) -> Result<T, Self::Error> {
        Self::decode(CdrReader::new(input))
    }
    fn deserialize_payload(payload: &zenoh::bytes::ZBytes) -> Result<Self::Output, Self::Error>
    where
        for<'a> &'a [u8]: Into<Self::Input<'a>>,
    {
        let buffer: ZBuf = payload.clone().into();
        Self::decode(CdrReader::from_zbuf(&buffer)?)
    }
}

impl crate::schema::MessageSchema for ZSlice {
    fn build_schema(
        builder: &mut crate::schema::SchemaBuilder,
    ) -> Result<ros_z_schema::TypeDef, ros_z_schema::SchemaError> {
        <Vec<u8> as crate::schema::MessageSchema>::build_schema(builder)
    }
}
impl Message for ZSlice {
    type Codec = CdrCodec<Self>;
    fn type_name() -> String {
        "ZSlice".into()
    }
    fn encode_cdr<B: CdrBuffer>(
        &self,
        writer: &mut CdrWriter<'_, LittleEndian, B>,
    ) -> ros_z_cdr::Result<()> {
        writer.write_u32(u32::try_from(self.len()).map_err(|_| {
            ros_z_cdr::Error::LengthOverflow {
                kind: "bytes",
                len: self.len(),
            }
        })?);
        writer.buffer_mut().append_zbuf(&ZBuf::from(self.clone()));
        Ok(())
    }
    fn decode_cdr(reader: &mut CdrReader<'_, LittleEndian>) -> ros_z_cdr::Result<Self> {
        let len = reader.read_sequence_length()?;
        reader.read_zslice(len)
    }
}
