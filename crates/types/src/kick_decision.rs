use coordinate_systems::Ground;
use linear_algebra::{Point2, Pose2};
use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

use crate::{motion_command::KickVariant, support_foot::Side};

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Serialize,
    Deserialize,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
)]
pub enum PlayingSituation {
    KickOff,
    CornerKick,
    PenaltyShot,
    #[default]
    Normal,
}

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct KickDecision {
    pub target: Point2<Ground>,
    pub variant: KickVariant,
    pub kicking_side: Side,
    pub kick_pose: Pose2<Ground>,
    pub strength: f32,
}

#[derive(
    Debug, Default, Clone, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect,
)]
pub struct DecisionParameters {
    pub distance_to_corner: f32,
    pub corner_kick_target_distance_to_goal: f32,
    pub max_kick_around_obstacle_angle: f32,
    pub kick_pose_robot_radius: f32,

    pub default_kick_variants: Vec<KickVariant>,
    pub corner_kick_variants: Vec<KickVariant>,
    pub kick_off_kick_variants: Vec<KickVariant>,
    pub penalty_shot_kick_variants: Vec<KickVariant>,

    pub default_kick_strength: f32,
    pub corner_kick_strength: f32,
    pub kick_off_kick_strength: f32,
    pub penalty_shot_kick_strength: f32,

    pub angle_distance_weight: f32,
    pub closer_to_goal_threshold: f32,
    pub goal_accuracy_margin: f32,
}
