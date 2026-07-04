use color_eyre::Result;
use coordinate_systems::{Field, Robot};
use linear_algebra::Isometry3;
use ros_z::{pubsub::Publisher, time::Time};
use types::{
    time_wrapper::TimeWrapper,
    visual_localization::{AssociationPoseHint, AssociationPoseHintSource},
};

use crate::visual_localization::association_pose_hint;

#[derive(Clone, Copy)]
pub(crate) struct LocalizationPublishers<'a> {
    localization: &'a Publisher<Option<Isometry3<Field, Robot>>>,
    pose_3d: &'a Publisher<TimeWrapper<Option<Isometry3<Field, Robot>>>>,
    association_pose_hint: &'a Publisher<TimeWrapper<Option<AssociationPoseHint>>>,
}

impl<'a> LocalizationPublishers<'a> {
    pub(crate) fn new(
        localization: &'a Publisher<Option<Isometry3<Field, Robot>>>,
        pose_3d: &'a Publisher<TimeWrapper<Option<Isometry3<Field, Robot>>>>,
        association_pose_hint: &'a Publisher<TimeWrapper<Option<AssociationPoseHint>>>,
    ) -> Self {
        Self {
            localization,
            pose_3d,
            association_pose_hint,
        }
    }

    pub(crate) async fn publish_outputs(
        self,
        time: Time,
        localization: Option<Isometry3<Field, Robot>>,
        pose_3d: Option<Isometry3<Field, Robot>>,
        source: AssociationPoseHintSource,
    ) -> Result<()> {
        self.localization.publish(&localization).await?;
        self.pose_3d
            .publish(&TimeWrapper {
                time,
                inner: pose_3d,
            })
            .await?;
        self.association_pose_hint
            .publish(&association_pose_hint(time, pose_3d, source))
            .await?;
        Ok(())
    }

    pub(crate) async fn publish_unlocalized(self, time: Time) -> Result<()> {
        self.localization.publish(&None).await?;
        self.pose_3d
            .publish(&TimeWrapper { time, inner: None })
            .await?;
        self.association_pose_hint
            .publish(&TimeWrapper { time, inner: None })
            .await?;
        Ok(())
    }
}
