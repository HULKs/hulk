use bevy::{camera::visibility::RenderLayers, prelude::*};

use super::{ball::BallAssets, goal::GoalAssets, object::ObjectKind, robot::RobotAssets};

#[derive(Resource)]
pub struct ObjectVisualAssets {
    pub ball: BallAssets,
    pub goal: GoalAssets,
    pub robot: RobotAssets,
}

impl FromWorld for ObjectVisualAssets {
    fn from_world(world: &mut World) -> Self {
        Self {
            ball: BallAssets::load(world),
            goal: GoalAssets::load(world),
            robot: RobotAssets::from_world(world),
        }
    }
}

impl ObjectVisualAssets {
    pub fn ground_offset(&self, kind: ObjectKind) -> f32 {
        match kind {
            ObjectKind::Ball => self.ball.radius(),
            ObjectKind::Robot => self.robot.ground_offset(),
            ObjectKind::Goal => 0.0,
        }
    }

    pub fn preview_center(&self, kind: ObjectKind) -> Vec3 {
        match kind {
            ObjectKind::Ball => Vec3::ZERO,
            ObjectKind::Robot => Vec3::new(0.0, -0.13, 0.0),
            ObjectKind::Goal => self.goal.preview_center(),
        }
    }

    pub fn preview_height(&self, kind: ObjectKind) -> f32 {
        match kind {
            ObjectKind::Ball => 0.25,
            ObjectKind::Robot => 1.05,
            ObjectKind::Goal => 1.05,
        }
    }

    pub fn spawn_preview(
        &self,
        kind: ObjectKind,
        commands: &mut Commands,
        transform: Transform,
        ghost: bool,
        layers: RenderLayers,
    ) -> Entity {
        match kind {
            ObjectKind::Ball => self.ball.spawn_visual(commands, transform, ghost, layers),
            ObjectKind::Robot => self.robot.spawn_visual(commands, transform, ghost, layers),
            ObjectKind::Goal => self.goal.spawn_visual(commands, transform, ghost, layers),
        }
    }
}
