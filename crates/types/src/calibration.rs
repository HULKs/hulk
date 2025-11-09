use serde::{Deserialize, Serialize};

use coordinate_systems::Ground;
use linear_algebra::Point2;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};

use crate::cycle_time::CycleTime;

#[derive(
    Copy, Clone, Debug, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct CalibrationCommand {
    pub target: Point2<Ground>,
    pub dispatch_time: CycleTime,
    pub capture: bool,
}

#[derive(
    Clone, Debug, Default, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct CalibrationCaptureResponse<Measurement>
where
    Measurement: Default,
{
    pub dispatch_time: CycleTime,
    pub measurement: Option<Measurement>,
}
