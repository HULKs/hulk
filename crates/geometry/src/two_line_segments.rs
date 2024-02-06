use std::collections::BTreeSet;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serialize_hierarchy::{Error, SerializeHierarchy};

use crate::line_segment::LineSegment;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(bound="")]
pub struct TwoLineSegments<Frame>(pub LineSegment<Frame>, pub LineSegment<Frame>);

// Manual implementation required because the derived version imposes Frame to be PartialEq
impl<Frame> PartialEq for TwoLineSegments<Frame> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 && self.1 == other.1
    }
}

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
