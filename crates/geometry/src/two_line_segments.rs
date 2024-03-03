use std::{cmp::PartialEq, collections::BTreeSet};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serialize_hierarchy::{Error, SerializeHierarchy};

use crate::line_segment::LineSegment;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct TwoLineSegments<Frame>(pub LineSegment<Frame>, pub LineSegment<Frame>);

impl<Frame> SerializeHierarchy for TwoLineSegments<Frame> {
    fn serialize_path<S>(&self, path: &str, _serializer: S) -> Result<S::Ok, Error<S::Error>>
    where
        S: Serializer,
    {
        Err(Error::TypeDoesNotSupportSerialization {
            type_name: "TwoLineSegments",
            path: path.to_string(),
        })
    }

    fn deserialize_path<'de, D>(
        &mut self,
        path: &str,
        _deserializer: D,
    ) -> Result<(), Error<D::Error>>
    where
        D: Deserializer<'de>,
    {
        Err(Error::TypeDoesNotSupportDeserialization {
            type_name: "TwoLineSegments",
            path: path.to_string(),
        })
    }

    fn exists(_path: &str) -> bool {
        false
    }

    fn get_fields() -> BTreeSet<String> {
        Default::default()
    }

    fn fill_fields(_fields: &mut BTreeSet<String>, _prefix: &str) {}
}
