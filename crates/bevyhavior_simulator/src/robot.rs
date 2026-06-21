use std::time::SystemTime;

use bevy::prelude::*;
use booster::FallDownState;
use color_eyre::Result;
use coordinate_systems::{Field, Ground, World};
use hsl_network_messages::PlayerNumber;
use linear_algebra::{Isometry2, Orientation2, Point2};
use types::{parameters::BehaviorParameters, primary_state::PrimaryState};

use crate::behavior_tree_simulator::SimulatorRobotBehavior;

#[derive(Component, Clone, Copy, Debug)]
pub struct SimulatorRobot {
    pub player_number: PlayerNumber,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct SimulatorGroundToWorld {
    pub ground_to_world: Isometry2<Ground, World>,
}

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct SimulatorHeadYaw {
    pub yaw: Orientation2<Ground>,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct SimulatorPrimaryState {
    pub primary_state: PrimaryState,
}

#[derive(Component, Clone, Debug)]
pub struct SimulatorRobotParameters {
    pub behavior: BehaviorParameters,
}

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct SimulatorFallDownState {
    pub fall_down_state: Option<FallDownState>,
}

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct SimulatorSuggestedSearchPosition {
    pub position: Option<Point2<Field>>,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct SimulatorLastKickTime {
    pub last_kick_time: SystemTime,
}

#[derive(Bundle)]
pub struct SimulatorRobotBundle {
    pub robot: SimulatorRobot,
    pub ground_to_world: SimulatorGroundToWorld,
    pub head_yaw: SimulatorHeadYaw,
    pub primary_state: SimulatorPrimaryState,
    pub behavior: SimulatorRobotBehavior,
    pub parameters: SimulatorRobotParameters,
    pub fall_down_state: SimulatorFallDownState,
    pub suggested_search_position: SimulatorSuggestedSearchPosition,
    pub last_kick_time: SimulatorLastKickTime,
}

impl SimulatorRobotBundle {
    pub fn new(
        player_number: PlayerNumber,
        ground_to_world: Isometry2<Ground, World>,
        parameters: BehaviorParameters,
    ) -> Result<Self> {
        Ok(Self {
            robot: SimulatorRobot { player_number },
            ground_to_world: SimulatorGroundToWorld { ground_to_world },
            head_yaw: SimulatorHeadYaw::default(),
            primary_state: SimulatorPrimaryState {
                primary_state: PrimaryState::Damping,
            },
            behavior: SimulatorRobotBehavior::new(parameters.clone()),
            parameters: SimulatorRobotParameters {
                behavior: parameters,
            },
            fall_down_state: SimulatorFallDownState::default(),
            suggested_search_position: SimulatorSuggestedSearchPosition::default(),
            last_kick_time: SimulatorLastKickTime {
                last_kick_time: SystemTime::UNIX_EPOCH,
            },
        })
    }

    pub fn with_primary_state(mut self, primary_state: PrimaryState) -> Self {
        self.primary_state.primary_state = primary_state;
        self
    }
}
