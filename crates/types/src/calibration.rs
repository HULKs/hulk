use coordinate_systems::Ground;
use linear_algebra::Point2;
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

use crate::{camera_position::CameraPosition, cycle_time::CycleTime};

#[derive(
    Clone, Debug, Default, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub enum CalibrationCommand {
    #[default]
    Inactive,
    Initialize {
        started_time: CycleTime,
    },
    LookAt {
        target: Point2<Ground>,
        camera: CameraPosition,
        dispatch_time: CycleTime,
    },
    Capture {
        camera: CameraPosition,
        dispatch_time: CycleTime,
    },
    Process,
    Finish,
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
