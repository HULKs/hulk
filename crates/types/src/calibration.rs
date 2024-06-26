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
    INACTIVE,
    INITIALIZE {
        started_time: CycleTime,
    },
    LOOKAT {
        target: Point2<Ground>,
        camera: Option<CameraPosition>,
        dispatch_time: CycleTime,
    },
    CAPTURE {
        dispatch_time: CycleTime,
    },
    PROCESS,
    FINISH,
}

#[derive(
    Clone, Debug, Default, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub enum CalibrationCaptureResponse<Measurement> {
    #[default]
    Idling,
    CommandRecieved {
        dispatch_time: CycleTime,
        output: Option<Measurement>,
    },
    RetriesExceeded {
        dispatch_time: CycleTime,
    },
}
