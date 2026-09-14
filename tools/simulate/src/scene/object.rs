use bevy::prelude::*;

use super::{ball, goal, visual::ObjectVisualAssets};
use crate::bevy_mujoco::MjcfObject;

#[derive(Clone, Copy, Component, Debug, Eq, PartialEq)]
pub enum ObjectKind {
    Ball,
    Robot,
    Goal,
}

impl ObjectKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ball => "Ball",
            Self::Robot => "K1 Robot",
            Self::Goal => "Goal",
        }
    }
}

#[derive(Component)]
pub struct ObjectPart(pub Entity);

pub fn spawn(
    kind: ObjectKind,
    commands: &mut Commands,
    assets: &ObjectVisualAssets,
    transform: Transform,
) -> Entity {
    match kind {
        ObjectKind::Ball => ball::spawn(commands, &assets.ball, transform),
        ObjectKind::Robot => super::robot::spawn(commands, &assets.robot, transform),
        ObjectKind::Goal => goal::spawn(commands, &assets.goal, transform),
    }
}

pub fn cleanup_parts(
    mut removed: RemovedComponents<MjcfObject>,
    parts: Query<(Entity, &ObjectPart)>,
    mut commands: Commands,
) {
    for owner in removed.read() {
        for (entity, part) in &parts {
            if part.0 == owner {
                commands.entity(entity).despawn();
            }
        }
    }
}
