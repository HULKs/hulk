use coordinate_systems::{Camera, Field, Pixel, Robot};
use linear_algebra::{Isometry3, Point2, Point3};
use ros_z::Message;
use serde::{Deserialize, Serialize};

/// How associations were obtained. Global recovery must preserve the existing
/// field-symmetry branch before its correspondences enter localization.
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, Message, PartialEq, Eq)]
pub enum VisualAssociationSource {
    #[default]
    Tracking,
    Global,
}

/// Tracking-only association input assembled from a coherent estimate and status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssociationGeometry {
    pub epoch: u64,
    pub generation: u64,
    pub estimate: crate::localization::PoseEstimate<Robot, Field>,
    pub last_successful_solve: ros_z::time::Time,
}

impl AssociationGeometry {
    /// Build association input from a coherent estimate and lifecycle snapshot.
    /// An old local frame must never be paired with a new epoch's state.
    pub fn from_estimate(
        estimate: &crate::localization::LocalizationEstimate,
        status: &crate::localization::LocalizationStatus,
    ) -> Option<Self> {
        use crate::localization::LocalizationState;
        if status.state != LocalizationState::Tracking
            || estimate.epoch != status.epoch
            || estimate.generation != status.generation
        {
            return None;
        }
        Some(Self {
            epoch: estimate.epoch,
            generation: estimate.generation,
            estimate: estimate.robot_to_field?,
            last_successful_solve: estimate.time,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
pub struct VisualLocalizationFrame {
    pub epoch: u64,
    pub generation: u64,
    pub source: VisualAssociationSource,
    pub robot_to_camera: Isometry3<Robot, Camera>,
    pub camera_intrinsic: projection::intrinsic::Intrinsic,
    pub associations: Vec<FieldMarkAssociation>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Message)]
pub struct FieldMarkAssociation {
    pub detection: Point2<Pixel>,
    pub field_point: Point3<Field>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
pub struct GlobalLocalizationDebug {
    pub association_count: usize,
    pub pairwise_distance_rms: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::localization::{
        LocalizationEstimate, LocalizationState, LocalizationStatus, PoseEstimate,
    };
    use ros_z::time::Time;

    #[test]
    fn tracking_geometry_requires_matching_state_epoch_generation_and_field_pose() {
        let mut estimate = LocalizationEstimate {
            generation: 0,
            time: Time::from_nanos(10),
            epoch: 3,
            robot_to_local: PoseEstimate {
                pose: Isometry3::identity(),
                covariance: nalgebra::SMatrix::identity(),
            },
            robot_to_field: Some(PoseEstimate {
                pose: Isometry3::identity(),
                covariance: nalgebra::SMatrix::identity(),
            }),
        };
        estimate.robot_to_local.pose.inner.translation.vector.x = 2.0;
        let field = estimate.robot_to_field.as_mut().unwrap();
        field.covariance[(0, 3)] = 0.125 + f64::EPSILON;
        field.covariance[(3, 0)] = 0.125 + f64::EPSILON;
        field.pose.inner.translation.x = 1.0 + f64::EPSILON;
        let mut status = LocalizationStatus {
            generation: 0,
            time: Time::from_nanos(20),
            epoch: 3,
            state: LocalizationState::LostTrack,
            heading: None,
        };
        assert!(AssociationGeometry::from_estimate(&estimate, &status).is_none());
        status.state = LocalizationState::Startup;
        assert!(AssociationGeometry::from_estimate(&estimate, &status).is_none());
        status.state = LocalizationState::Tracking;
        let geometry = AssociationGeometry::from_estimate(&estimate, &status).unwrap();
        assert_eq!(geometry.last_successful_solve, estimate.time);
        assert_eq!(geometry.estimate, estimate.robot_to_field.unwrap());
        status.epoch = 4;
        assert!(AssociationGeometry::from_estimate(&estimate, &status).is_none());
        status.epoch = estimate.epoch;
        status.generation += 1;
        assert!(AssociationGeometry::from_estimate(&estimate, &status).is_none());
        status.generation = estimate.generation;
        estimate.robot_to_field = None;
        assert!(AssociationGeometry::from_estimate(&estimate, &status).is_none());
    }
}
