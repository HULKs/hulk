use std::collections::HashSet;

use coordinate_systems::Framed;
use nalgebra::Point2;
use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use crate::{
    coordinate_systems::{Ground, Pixel},
    line::Line2,
};

#[derive(Clone, Default, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub struct LineData {
    pub lines_in_ground: Vec<Line2<Ground>>,
    pub used_segments: HashSet<Framed<Pixel, Point2<u16>>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub enum LineDiscardReason {
    TooFewPoints,
    LineTooShort,
    LineTooLong,
    TooFarAway,
}
