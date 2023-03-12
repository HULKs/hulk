use serde::{
    de::{self, Error},
    Deserialize, Deserializer, Serialize, Serializer,
};
use serialize_hierarchy::SerializeHierarchy;
use std::{collections::BTreeSet, ops::Index, sync::Arc};

use image::{load_from_memory_with_format, ImageFormat};
use nalgebra::Point2;

use super::color::YCbCr422;

// #[cfg(test)]
// mod tests {
//     use serde_test::{assert_tokens, Configure, Token};
//     use serialize_hierarchy::bincode::{deserialize, serialize};

//     use super::*;

//     #[test]
//     fn image_has_zero_constructor() {
//         let image = NaoImage::zero(10, 12);
//         assert!(image.buffer.iter().all(|&x| x == YCbCr422::default()));
//     }

//     #[test]
//     fn image_has_width_and_height() {
//         let image = NaoImage::zero(10, 12);
//         assert_eq!(image.width(), 10);
//         assert_eq!(image.height(), 12);
//     }

//     #[test]
//     fn image_can_be_indexed() {
//         let image = NaoImage::from_ycbcr_buffer(
//             2,
//             2,
//             vec![
//                 Default::default(),
//                 Default::default(),
//                 Default::default(),
//                 YCbCr422 {
//                     y1: 1,
//                     cb: 2,
//                     y2: 3,
//                     cr: 4,
//                 },
//             ],
//         );
//         assert_eq!(image.at(2, 1), YCbCr444 { y: 1, cb: 2, cr: 4 });
//     }

//     #[derive(Debug, Deserialize, Serialize)]
//     struct ImageTestingWrapper(NaoImage);

//     impl PartialEq for ImageTestingWrapper {
//         fn eq(&self, other: &Self) -> bool {
//             let buffers_are_equal = self.0.buffer == other.0.buffer;
//             self.0.width_422 == other.0.width_422
//                 && self.0.height == other.0.height
//                 && buffers_are_equal
//         }
//     }

//     #[test]
//     fn readable_image_serialization() {
//         let image = ImageTestingWrapper(NaoImage {
//             width_422: 1,
//             height: 1,
//             buffer: Arc::new(vec![YCbCr422 {
//                 y1: 0,
//                 cb: 1,
//                 y2: 2,
//                 cr: 3,
//             }]),
//         });

//         assert_tokens(
//             &image.readable(),
//             &[
//                 Token::NewtypeStruct {
//                     name: "ImageTestingWrapper",
//                 },
//                 Token::Struct {
//                     name: "NaoImage",
//                     len: 3,
//                 },
//                 Token::Str("width_422"),
//                 Token::U32(1),
//                 Token::Str("height"),
//                 Token::U32(1),
//                 Token::Str("buffer"),
//                 Token::Seq { len: Some(1) },
//                 Token::Struct {
//                     name: "YCbCr422",
//                     len: 4,
//                 },
//                 Token::Str("y1"),
//                 Token::U8(0),
//                 Token::Str("cb"),
//                 Token::U8(1),
//                 Token::Str("y2"),
//                 Token::U8(2),
//                 Token::Str("cr"),
//                 Token::U8(3),
//                 Token::StructEnd,
//                 Token::SeqEnd,
//                 Token::StructEnd,
//             ],
//         );
//     }

//     #[test]
//     fn compact_serialization_and_deserialization_result_in_equality() {
//         let image = ImageTestingWrapper(NaoImage {
//             width_422: 1,
//             height: 1,
//             buffer: Arc::new(vec![YCbCr422 {
//                 y1: 63,
//                 cb: 127,
//                 y2: 191,
//                 cr: 255,
//             }]),
//         });

//         let deserialized_serialized_image: ImageTestingWrapper =
//             deserialize(&serialize(&image).unwrap()).unwrap();
//         assert_eq!(deserialized_serialized_image, image);
//     }
// }
