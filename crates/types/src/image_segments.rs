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

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
