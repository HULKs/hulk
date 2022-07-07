use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use super::{Intensity, YCbCr444};

#[derive(Default, Clone, Debug, Deserialize, Serialize, SerializeHierarchy)]
pub struct ImageSegments {
    pub scan_grid: ScanGrid,
}

#[derive(Default, Clone, Debug, Deserialize, Serialize, SerializeHierarchy)]
pub struct ScanGrid {
    pub vertical_scan_lines: Vec<ScanLine>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct ScanLine {
    pub position: u16,
    pub segments: Vec<Segment>,
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
pub struct Segment {
    pub start: u16,
    pub end: u16,
    #[allow(dead_code)]
    pub start_edge_type: EdgeType,
    #[allow(dead_code)]
    pub end_edge_type: EdgeType,
    pub color: YCbCr444,
    pub field_color: Intensity,
}

impl Segment {
    #[allow(dead_code)]
    pub fn center(&self) -> u16 {
        (self.length() >> 1) + self.start
    }

    pub fn length(&self) -> u16 {
        self.end - self.start
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
#[allow(dead_code)]
pub enum EdgeType {
    Rising,
    Falling,
    ImageBorder,
    LimbBorder,
}
