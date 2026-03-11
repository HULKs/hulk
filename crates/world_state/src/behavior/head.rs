use types::{
    motion_command::{HeadMotion, ImageRegion},
    roles::Role,
    world_state::WorldState,
};

#[derive(Debug)]
pub struct LookAction<'cycle> {
    world_state: &'cycle WorldState,
}

impl<'cycle> LookAction<'cycle> {
    pub fn new(world_state: &'cycle WorldState) -> Self {
        Self { world_state }
    }

    pub fn execute(&self) -> HeadMotion {
        if self.world_state.robot.role == Role::Keeper
            || self.world_state.robot.role == Role::ReplacementKeeper
        {
            if let Some(target) = self.world_state.ball.and_then(|ball_position| {
                if ball_position.ball_in_field.x() <= 0.0 {
                    Some(ball_position.ball_in_ground)
                } else {
                    None
                }
            }) {
                HeadMotion::LookAt {
                    target,
                    image_region_target: ImageRegion::Center,
                }
            } else {
                HeadMotion::Center {
                    image_region_target: ImageRegion::Top,
                }
            }
        } else {
            HeadMotion::Center {
                image_region_target: ImageRegion::Top,
            }
        }
    }
}
