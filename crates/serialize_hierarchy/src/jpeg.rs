use std::collections::BTreeSet;

use image::ImageError;
use serde::{de, ser, Deserialize, Deserializer, Serialize, Serializer};

use crate::{error::Error, SerializeHierarchy};

const SERIALIZATION_JPEG_QUALITY: u8 = 40;

pub trait EncodeJpeg {
    fn encode_as_jpeg(&self, quality: u8) -> Result<Vec<u8>, ImageError>;
}

pub trait DecodeJpeg
where
    Self: Sized,
{
    fn decode_from_jpeg(jpeg: Vec<u8>) -> Result<Self, ImageError>;
}

// impl<T> SerializeHierarchy for T
// where
//     T: EncodeJpeg + DecodeJpeg,
// {
//     fn serialize_path<S>(&self, path: &str, serializer: S) -> Result<S::Ok, Error<S::Error>>
//     where
//         S: Serializer,
//     {
//         match path {
//             "jpeg" => self
//                 .encode_as_jpeg(SERIALIZATION_JPEG_QUALITY)
//                 .map_err(|error| Error::SerializationFailed(ser::Error::custom(error)))?
//                 .serialize(serializer)
//                 .map_err(Error::SerializationFailed),
//             _ => Err(Error::UnexpectedPathSegment {
//                 segment: path.to_string(),
//             }),
//         }
//     }

//     fn deserialize_path<'de, D>(
//         &mut self,
//         path: &str,
//         deserializer: D,
//     ) -> Result<(), Error<D::Error>>
//     where
//         D: Deserializer<'de>,
//         <D as Deserializer<'de>>::Error: de::Error,
//     {
//         match path {
//             "jpeg" => {
//                 let jpeg_buffer =
//                     Vec::<u8>::deserialize(deserializer).map_err(Error::DeserializationFailed)?;
//                 *self = T::decode_from_jpeg(jpeg_buffer)
//                     .map_err(|error| Error::DeserializationFailed(de::Error::custom(error)))?;
//                 Ok(())
//             }
//             _ => Err(Error::UnexpectedPathSegment {
//                 segment: path.to_string(),
//             }),
//         }
//     }

//     fn exists(path: &str) -> bool {
//         matches!(path, "jpeg")
//     }

//     fn get_fields() -> BTreeSet<String> {
//         ["jpeg".to_string()].into_iter().collect()
//     }
// }
