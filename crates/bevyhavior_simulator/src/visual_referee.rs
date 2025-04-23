use std::time::Duration;

use bevy::prelude::*;
use spl_network_messages::Team;
use types::{field_dimensions::GlobalFieldSide, pose_kinds::PoseKind};

const FREE_KICK_POSE_DURATION: Duration = Duration::from_secs(5);

#[derive(Resource, Default)]
pub struct VisualRefereeResource {
    pub pose_kind: Option<PoseKind>,
    pub last_pose_timer: Option<Duration>,
}

impl VisualRefereeResource {
    pub fn reset(&mut self) {
        *self = Default::default();
    }

    pub fn update_visual_referee(&mut self, time: Time) {
        if self
            .last_pose_timer
            .as_ref()
            .is_some_and(|last_pose_timer| {
                time.elapsed() - *last_pose_timer > FREE_KICK_POSE_DURATION
            })
        {
            self.pose_kind = None;
            self.last_pose_timer = None;
        }
    }

    pub fn start_free_kick_pose(
        &mut self,
        time: Time,
        kicking_team: Team,
        hulks_global_field_side: GlobalFieldSide,
    ) {
        self.last_pose_timer = Some(time.elapsed());
        self.pose_kind = Some(PoseKind::FreeKick {
            global_field_side: match kicking_team {
                Team::Hulks => hulks_global_field_side.mirror(),
                Team::Opponent => hulks_global_field_side,
            },
        });
    }
}
