use color_eyre::Result;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

use crate::bounding_box::BoundingBox;

#[derive(Debug, Clone, Serialize, Deserialize, PathIntrospect, PathSerialize, PathDeserialize)]
pub struct Detection {
    pub label: String,
    pub bounding_box: BoundingBox,
}
