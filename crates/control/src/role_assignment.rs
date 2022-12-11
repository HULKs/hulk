use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use nalgebra::{Isometry2, Point2};
use spl_network_messages::PlayerNumber;
use types::{
    configuration::SplNetwork, BallPosition, FallState, FieldDimensions, GameControllerState,
    PrimaryState, Role, SensorData,
};

pub struct RoleAssignment {}

#[context]
pub struct CreationContext {
    pub field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    pub forced_role: Parameter<Option<Role>, "control.role_assignment.forced_role?">,
    pub player_number: Parameter<PlayerNumber, "player_number">,
    pub spl_network: Parameter<SplNetwork, "spl_network">,
}

#[context]
pub struct CycleContext {
    pub ball_position: RequiredInput<Option<BallPosition>, "ball_position?">,
    pub fall_state: RequiredInput<Option<FallState>, "fall_state?">,
    pub game_controller_state: RequiredInput<Option<GameControllerState>, "game_controller_state?">,
    pub primary_state: RequiredInput<Option<PrimaryState>, "primary_state?">,
    pub robot_to_field: RequiredInput<Option<Isometry2<f32>>, "robot_to_field?">,
    pub sensor_data: Input<SensorData, "sensor_data">,

    pub field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    pub forced_role: Parameter<Option<Role>, "control.role_assignment.forced_role?">,
    pub player_number: Parameter<PlayerNumber, "player_number">,
    pub spl_network: Parameter<SplNetwork, "spl_network">,
    // TODO: wieder einkommentieren
    // pub spl_message: PerceptionInput<SplMessage, "SplNetwork", "spl_message">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub team_ball: MainOutput<Option<BallPosition>>,
    // pub message_receivers: MainOutput<MessageReceivers>,
    pub network_robot_obstacles: MainOutput<Option<Vec<Point2<f32>>>>,
    pub role: MainOutput<Option<Role>>,
}

impl RoleAssignment {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
