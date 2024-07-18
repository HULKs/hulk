use coordinate_systems::Pixel;
use linear_algebra::{point, Point2};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

use crate::color::{Intensity, YCbCr444};

#[derive(
    Default, Clone, Debug, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct ImageSegments {
    pub scan_grid: ScanGrid,
}

#[derive(
    Default, Clone, Debug, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct ScanGrid {
    pub horizontal_scan_lines: Vec<ScanLine>,
    pub vertical_scan_lines: Vec<ScanLine>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ScanLine {
    pub position: u16,
    pub segments: Vec<Segment>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Segment {
    pub start: u16,
    pub end: u16,
    pub start_edge_type: EdgeType,
    pub end_edge_type: EdgeType,
    pub color: YCbCr444,
    pub field_color: Intensity,
}

impl Segment {
    pub fn center(&self) -> u16 {
        (self.length() >> 1) + self.start
    }

    pub fn length(&self) -> u16 {
        self.end - self.start
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    Deserialize,
    Eq,
    PartialEq,
    Serialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub enum EdgeType {
    Rising,
    Falling,
    ImageBorder,
    LimbBorder,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Direction {
    Horizontal,
    Vertical,
}

#[derive(
    Clone, Copy, Debug, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct GenericSegment {
    pub start: Point2<Pixel, u16>,
    pub end: Point2<Pixel, u16>,
    pub start_edge_type: EdgeType,
    pub end_edge_type: EdgeType,
}

impl GenericSegment {
    pub fn center(&self) -> Point2<Pixel, u16> {
        point![
            self.start.x() + (self.end.x() - self.start.x()) / 2,
            self.start.y() + (self.end.y() - self.start.y()) / 2,
        ]
    }
}
