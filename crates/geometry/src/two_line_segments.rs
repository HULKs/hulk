use std::{cmp::PartialEq, collections::BTreeSet};

use path_serde::{deserialize, serialize, PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::line_segment::LineSegment;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct TwoLineSegments<Frame>(pub LineSegment<Frame>, pub LineSegment<Frame>);

impl<Frame> PathSerialize for TwoLineSegments<Frame> {
    fn serialize_path<S>(
        &self,
        path: &str,
        _serializer: S,
    ) -> Result<S::Ok, serialize::Error<S::Error>>
    where
        S: Serializer,
    {
        Err(serialize::Error::NotSupported {
            type_name: "TwoLineSegments",
            path: path.to_string(),
        })
    }
}

impl<Frame> PathDeserialize for TwoLineSegments<Frame> {
    fn deserialize_path<'de, D>(
        &mut self,
        path: &str,
        _deserializer: D,
    ) -> Result<(), deserialize::Error<D::Error>>
    where
        D: Deserializer<'de>,
    {
        Err(deserialize::Error::NotSupported {
            type_name: "TwoLineSegments",
            path: path.to_string(),
        })
    }
}

impl<Frame> PathIntrospect for TwoLineSegments<Frame> {
    fn extend_with_fields(_fields: &mut BTreeSet<String>, _prefix: &str) {}
}
