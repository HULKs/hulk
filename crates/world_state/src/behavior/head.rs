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
            let target = self
                .world_state
                .ball
                .and_then(|ball_position| {
                    if ball_position.ball_in_field.x() <= 0.0 {
                        Some(ball_position.ball_in_ground)
                    } else {
                        None
                    }
                })
                .unwrap_or(self.world_state.position_of_interest);

            HeadMotion::LookAt {
                target,
                image_region_target: ImageRegion::Center,
            }
        } else {
            HeadMotion::LookAt {
                target: self.world_state.position_of_interest,
                image_region_target: ImageRegion::Center,
            }
        }
    }
}
