use macros::SerializeHierarchy;
use serde::{Deserialize, Serialize};

use super::image_segments::ScanGrid;

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct FilteredSegments {
    pub scan_grid: ScanGrid,
}
