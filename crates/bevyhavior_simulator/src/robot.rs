use std::time::SystemTime;

use bevy::prelude::*;
use booster::FallDownState;
use color_eyre::Result;
use coordinate_systems::{Field, Ground, World};
use hsl_network_messages::{PlayerNumber, Team};
use linear_algebra::{Isometry2, Orientation2, Point2};
use serde::Serializer;
use types::{parameters::BehaviorParameters, primary_state::PrimaryState};

use crate::behavior_tree_simulator::SimulatorRobotBehavior;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SimulatorRobotId {
    pub team: Team,
    pub player_number: PlayerNumber,
}

impl SimulatorRobotId {
    fn team_order(self) -> u8 {
        match self.team {
            Team::Hulks => 0,
            Team::Opponent => 1,
        }
    }
}

impl Ord for SimulatorRobotId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.team_order(), self.player_number).cmp(&(other.team_order(), other.player_number))
    }
}

impl PartialOrd for SimulatorRobotId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::hash::Hash for SimulatorRobotId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.team_order().hash(state);
        self.player_number.hash(state);
    }
}

impl SimulatorRobotId {
    pub fn new(team: Team, player_number: PlayerNumber) -> Self {
        Self {
            team,
            player_number,
        }
    }
}

impl std::fmt::Display for SimulatorRobotId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let team = match self.team {
            Team::Hulks => "H",
            Team::Opponent => "O",
        };
        write!(formatter, "{team}{}", self.player_number)
    }
}

impl serde::Serialize for SimulatorRobotId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Component, Clone, Copy, Debug)]
pub struct SimulatorRobot {
    pub team: Team,
    pub player_number: PlayerNumber,
}

impl SimulatorRobot {
    pub fn id(&self) -> SimulatorRobotId {
        SimulatorRobotId::new(self.team, self.player_number)
    }
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
        team: Team,
        player_number: PlayerNumber,
        ground_to_world: Isometry2<Ground, World>,
        parameters: BehaviorParameters,
    ) -> Result<Self> {
        Ok(Self {
            robot: SimulatorRobot {
                team,
                player_number,
            },
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
