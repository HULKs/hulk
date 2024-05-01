use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

use super::image_segments::ScanGrid;

#[derive(
    Clone, Debug, Default, Deserialize, Serialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct FilteredSegments {
    pub scan_grid: ScanGrid,
}
