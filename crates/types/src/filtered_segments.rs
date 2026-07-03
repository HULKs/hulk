use serde::{Deserialize, Serialize};

use super::image_segments::ScanGrid;

#[derive(Clone, Debug, Default, Deserialize, Serialize, ros_z::Message)]
pub struct FilteredSegments {
    pub scan_grid: ScanGrid,
}
