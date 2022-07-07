use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

use super::image_segments::ScanGrid;

#[derive(Clone, Debug, Default, Deserialize, Serialize, SerializeHierarchy)]
pub struct FilteredSegments {
    pub scan_grid: ScanGrid,
}
