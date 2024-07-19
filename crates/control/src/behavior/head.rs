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
        if self.world_state.robot.role == Role::Keeper {
            if let Some(ball_position) = self.world_state.ball {
                HeadMotion::LookAt {
                    target: ball_position.ball_in_ground,
                    image_region_target: ImageRegion::Center,
                    camera: None,
                }
            } else {
                HeadMotion::LookAt {
                    target: self.world_state.position_of_interest,
                    image_region_target: ImageRegion::Center,
                    camera: None,
                }
            }
        } else {
            HeadMotion::LookAt {
                target: self.world_state.position_of_interest,
                image_region_target: ImageRegion::Center,
                camera: None,
            }
        }
    }
}
