use std::time::SystemTime;

use booster::ImuState;
use coordinate_systems::{Field, Pixel};
use factrs::core::SE3;
use linear_algebra::{Point2 as FramedPoint2, Point3 as FramedPoint3};
use nalgebra::{Point2, Point3};

use crate::factors::{
    foot_above_ground::FootHeightMeasurement, visual_odometry::VisualOdometryMeasurement,
};

#[derive(Debug, Clone)]
pub enum SensorMeasurement {
    Imu(ImuMeasurement),
    GlobalPose(GlobalPoseMeasurement),
    Visual(Vec<VisualReprojectionMeasurement>),
    PoseHintVisual(Vec<VisualReprojectionMeasurement>),
    VisualOdometry(VisualOdometryMeasurement),
    FootHeights(FootHeightMeasurement),
}

impl SensorMeasurement {
    pub fn time(&self) -> SystemTime {
        match self {
            SensorMeasurement::Imu(imu) => imu.time,
            SensorMeasurement::GlobalPose(global_pose) => global_pose.time,
            SensorMeasurement::Visual(visual) | SensorMeasurement::PoseHintVisual(visual) => {
                visual
                    .first()
                    .expect("visual frames must contain at least one measurement")
                    .time
            }
            SensorMeasurement::VisualOdometry(visual_odometry) => visual_odometry.current_time,
            SensorMeasurement::FootHeights(foot_heights) => foot_heights.time,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ImuMeasurement {
    pub time: SystemTime,
    pub state: ImuState,
}

#[derive(Debug, Clone)]
pub struct GlobalPoseMeasurement {
    pub time: SystemTime,
    pub robot_to_field: SE3<f64>,
}

/// A fixed visual feature association in domain frames.
#[derive(Debug, Clone, Copy)]
pub struct VisualReprojectionAssociation {
    /// Detected feature location in pixel coordinates.
    pub detection: FramedPoint2<Pixel>,
    /// Associated field feature in field coordinates.
    pub field_point: FramedPoint3<Field>,
    /// Association source, used to select backend weighting and robustification.
    pub kind: VisualReprojectionAssociationKind,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum VisualReprojectionAssociationKind {
    /// Globally certified, unique association.
    GlobalUnique,
    /// Association selected from a trusted current pose hint.
    PoseHint,
}

#[derive(Debug, Clone)]
pub struct VisualReprojectionMeasurement {
    /// Time of the detection
    pub time: SystemTime,
    /// The detected feature in image space.
    pub detection: Point2<f64>,
    /// The associated 3d field point.
    pub field_point: Point3<f64>,
    /// Transformation from the robot frame to the camera frame
    pub robot_to_camera: SE3<f64>,
}
