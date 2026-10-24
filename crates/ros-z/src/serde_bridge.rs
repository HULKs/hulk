//! Serde-backed field methods for handwritten [`crate::Message`] implementations.
//! These share the enclosing CDR reader/writer and alignment; they are not a codec.
use ros_z_cdr::{CdrBuffer, CdrReader, CdrWriter, LittleEndian, SerdeCdrSerializer};
use serde::{Serialize, de::DeserializeOwned};

/// Encode one Serde-backed field at the enclosing writer's current alignment.
pub fn encode_serde<T: Serialize + ?Sized, B: CdrBuffer>(
    input: &T,
    writer: &mut CdrWriter<'_, LittleEndian, B>,
) -> ros_z_cdr::Result<()> {
    input.serialize(&mut SerdeCdrSerializer::from_writer(writer.reborrow()))
}

/// Decode one Serde-backed field, updating the reader position even on failure.
pub fn decode_serde<T: DeserializeOwned>(
    reader: &mut CdrReader<'_, LittleEndian>,
) -> ros_z_cdr::Result<T> {
    let mut deserializer =
        ros_z_cdr::CdrDeserializer::from_reader(std::mem::replace(reader, CdrReader::new(&[])));
    let result = T::deserialize(&mut deserializer);
    *reader = deserializer.into_reader();
    result
}
