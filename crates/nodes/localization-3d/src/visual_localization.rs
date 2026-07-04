use color_eyre::Result;
use coordinate_systems::{Camera, Field, Robot};
use linear_algebra::Isometry3;
use localization_factrs::{
    VinsFrontend, VinsFrontendError, VisualReprojectionAssociation,
    VisualReprojectionAssociationKind,
};
use ros_z::time::Time;
use types::{
    time_wrapper::TimeWrapper,
    visual_localization::{
        AssociationPoseHint, AssociationPoseHintSource, FieldMarkAssociation,
        FieldMarkAssociationSource, VisualLocalizationFrame,
    },
};

use crate::live_odometry::LiveOdometryLocalization;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum GlobalVisualLock {
    Unlocked,
    WaitingForBackend,
    Locked,
}

impl GlobalVisualLock {
    pub(crate) fn is_active(self) -> bool {
        !matches!(self, Self::Unlocked)
    }

    pub(crate) fn has_backend_result(self) -> bool {
        matches!(self, Self::Locked)
    }

    pub(crate) fn accepts_association_frame(self, has_global_associations: bool) -> bool {
        !has_global_associations || self.has_backend_result()
    }

    pub(crate) fn should_ingest_backend_reset(self) -> bool {
        !matches!(self, Self::WaitingForBackend)
    }

    pub(crate) fn mark_waiting_for_backend(&mut self) {
        *self = Self::WaitingForBackend;
    }

    pub(crate) fn mark_backend_result(&mut self) -> bool {
        if !self.is_active() {
            return false;
        }
        *self = Self::Locked;
        true
    }

    pub(crate) fn reset_for_damping(&mut self) {
        *self = Self::Unlocked;
    }
}

pub(crate) fn association_pose_hint(
    time: Time,
    field_to_robot: Option<Isometry3<Field, Robot>>,
    source: AssociationPoseHintSource,
) -> TimeWrapper<Option<AssociationPoseHint>> {
    TimeWrapper {
        time,
        inner: field_to_robot.map(|pose| AssociationPoseHint {
            robot_to_field: pose.inverse(),
            source,
        }),
    }
}

pub(crate) fn handle_visual_localization_frame(
    frontend: &mut VinsFrontend,
    live_localization: &mut LiveOdometryLocalization,
    global_visual_lock: &mut GlobalVisualLock,
    frame: TimeWrapper<VisualLocalizationFrame>,
) -> Result<(), VinsFrontendError> {
    let TimeWrapper { time, inner } = frame;
    let VisualLocalizationFrame {
        robot_to_camera,
        associations,
        backend_reset,
    } = inner;

    if let Some(robot_to_field) = backend_reset {
        if !global_visual_lock.should_ingest_backend_reset() {
            return Ok(());
        }

        frontend.ingest_global_pose(time.to_wallclock(), robot_to_field)?;
        global_visual_lock.mark_waiting_for_backend();
        live_localization.clear();
        return Ok(());
    }

    if !global_visual_lock.accepts_association_frame(has_global_associations(&associations)) {
        return Ok(());
    }

    ingest_visual_localization_associations(frontend, time, robot_to_camera, associations)
}

fn has_global_associations(associations: &[FieldMarkAssociation]) -> bool {
    associations
        .iter()
        .any(|association| matches!(association.source, FieldMarkAssociationSource::GlobalUnique))
}

fn ingest_visual_localization_associations(
    frontend: &mut VinsFrontend,
    time: Time,
    robot_to_camera: Isometry3<Robot, Camera>,
    associations: Vec<FieldMarkAssociation>,
) -> Result<(), VinsFrontendError> {
    let associations = associations
        .into_iter()
        .map(|association| VisualReprojectionAssociation {
            detection: association.detection,
            field_point: association.field_point,
            kind: match association.source {
                FieldMarkAssociationSource::GlobalUnique => {
                    VisualReprojectionAssociationKind::GlobalUnique
                }
                FieldMarkAssociationSource::PoseHint => VisualReprojectionAssociationKind::PoseHint,
            },
        });
    frontend.ingest_visual_reprojection_associations(
        time.to_wallclock(),
        associations,
        robot_to_camera,
    )
}
