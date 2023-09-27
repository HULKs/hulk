use types::{motion_command::HeadMotion, world_state::WorldState};

#[derive(Debug)]
pub struct LookAction<'cycle> {
    world_state: &'cycle WorldState,
}

impl<'cycle> LookAction<'cycle> {
    pub fn new(world_state: &'cycle WorldState) -> Self {
        Self { world_state }
    }

    pub fn execute(&self) -> HeadMotion {
        HeadMotion::LookAt {
            target: self.world_state.position_of_interest,
            camera: None,
        }
    }
}
