use context_attribute::context;
use framework::{MainOutput, OptionalInput, Parameter};
use nalgebra::{Isometry2, Point2};
use spl_network_messages::PlayerNumber;
use types::{
    configuration::SplNetwork, BallPosition, FallState, FieldDimensions, GameControllerState,
    PrimaryState, Role, SensorData,
};

pub struct RoleAssignment {}

#[context]
pub struct NewContext {
    pub field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    pub forced_role: Parameter<Option<Role>, "control/role_assignment/forced_role">,
    pub player_number: Parameter<PlayerNumber, "player_number">,
    pub spl_network: Parameter<SplNetwork, "spl_network">,
}

#[context]
pub struct CycleContext {
    pub ball_position: OptionalInput<BallPosition, "ball_position?">,
    pub fall_state: OptionalInput<FallState, "fall_state?">,
    pub game_controller_state: OptionalInput<Option<GameControllerState>, "game_controller_state?">,
    pub primary_state: OptionalInput<PrimaryState, "primary_state?">,
    pub robot_to_field: OptionalInput<Isometry2<f32>, "robot_to_field?">,
    pub sensor_data: OptionalInput<SensorData, "sensor_data?">,

    pub field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    pub forced_role: Parameter<Option<Role>, "control/role_assignment/forced_role">,
    pub player_number: Parameter<PlayerNumber, "player_number">,
    pub spl_network: Parameter<SplNetwork, "spl_network">,
    // TODO: wieder einkommentieren
    // pub spl_message: PerceptionInput<SplMessage, "SplNetwork", "spl_message">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub team_ball: MainOutput<BallPosition>,
    // pub message_receivers: MainOutput<MessageReceivers>,
    pub network_robot_obstacles: MainOutput<Vec<Point2<f32>>>,
    pub role: MainOutput<Role>,
}

impl RoleAssignment {
    pub fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
