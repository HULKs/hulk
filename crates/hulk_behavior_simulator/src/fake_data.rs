use std::{net::SocketAddr, time::Duration};

use color_eyre::Result;
use linear_algebra::{Isometry2, Isometry3, Orientation3, Point2};
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::{Field, Ground, Robot};
use framework::MainOutput;
use spl_network_messages::HulkMessage;
use types::{
    ball_position::{BallPosition, HypotheticalBallPosition},
    buttons::Buttons,
    calibration::CalibrationCommand,
    cycle_time::CycleTime,
    fall_state::FallState,
    filtered_whistle::FilteredWhistle,
    game_controller_state::GameControllerState,
    joints::head::HeadJoints,
    obstacle_avoiding_arms::ArmCommands,
    obstacles::Obstacle,
    parameters::{BallFilterParameters, CameraMatrixParameters},
    penalty_shot_direction::PenaltyShotDirection,
    sensor_data::SensorData,
    support_foot::SupportFoot,
};

use walking_engine::parameters::Parameters as WalkingEngineParameters;

use crate::interfake::FakeDataInterface;

#[derive(Deserialize, Serialize)]
pub struct FakeData {}

#[context]
#[allow(dead_code)]
pub struct CreationContext {
    maximum_velocity: Parameter<HeadJoints<f32>, "head_motion.maximum_velocity">,
    top_camera_matrix_parameters:
        Parameter<CameraMatrixParameters, "camera_matrix_parameters.vision_top">,
    ball_filter: Parameter<BallFilterParameters, "ball_filter">,
    glance_angle: Parameter<f32, "look_at.glance_angle">,
    parameters: Parameter<WalkingEngineParameters, "walking_engine">,
}

#[context]
#[allow(dead_code)]
pub struct CycleContext {
    hardware_interface: HardwareInterface,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub ball_position: MainOutput<Option<BallPosition<Ground>>>,
    pub buttons: MainOutput<Buttons>,
    pub cycle_time: MainOutput<CycleTime>,
    pub fall_state: MainOutput<FallState>,
    pub filtered_whistle: MainOutput<FilteredWhistle>,
    pub game_controller_address: MainOutput<Option<SocketAddr>>,
    pub game_controller_state: MainOutput<Option<GameControllerState>>,
    pub ground_to_field: MainOutput<Option<Isometry2<Ground, Field>>>,
    pub has_ground_contact: MainOutput<bool>,
    pub hulk_messages: MainOutput<Vec<HulkMessage>>,
    pub majority_vote_is_referee_ready_pose_detected: MainOutput<bool>,
    pub visual_referee_proceed_to_ready: MainOutput<bool>,
    pub hypothetical_ball_positions: MainOutput<Vec<HypotheticalBallPosition<Ground>>>,
    pub is_localization_converged: MainOutput<bool>,
    pub obstacles: MainOutput<Vec<Obstacle>>,
    pub penalty_shot_direction: MainOutput<Option<PenaltyShotDirection>>,
    pub sensor_data: MainOutput<SensorData>,
    pub stand_up_back_estimated_remaining_duration: MainOutput<Option<Duration>>,
    pub calibration_command: MainOutput<Option<CalibrationCommand>>,
    pub stand_up_front_estimated_remaining_duration: MainOutput<Option<Duration>>,
    pub robot_to_ground: MainOutput<Option<Isometry3<Robot, Ground>>>,
    pub robot_orientation: MainOutput<Option<Orientation3<Field>>>,
    pub obstacle_avoiding_arms: MainOutput<ArmCommands>,
    pub zero_moment_point: MainOutput<Point2<Ground>>,
    pub number_of_consecutive_cycles_zero_moment_point_outside_support_polygon: MainOutput<i32>,
    pub support_foot: MainOutput<SupportFoot>,
}

impl FakeData {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext<impl FakeDataInterface>) -> Result<MainOutputs> {
        let mut receiver = context
            .hardware_interface
            .get_last_database_receiver()
            .lock();
        let last_database = &receiver.borrow_and_mark_as_seen().main_outputs;
        Ok(MainOutputs {
            ball_position: last_database.ball_position.into(),
            buttons: last_database.buttons.into(),
            cycle_time: last_database.cycle_time.into(),
            fall_state: last_database.fall_state.into(),
            filtered_whistle: last_database.filtered_whistle.clone().into(),
            game_controller_state: last_database.game_controller_state.clone().into(),
            game_controller_address: last_database.game_controller_address.into(),
            has_ground_contact: last_database.has_ground_contact.into(),
            hulk_messages: last_database.hulk_messages.clone().into(),
            majority_vote_is_referee_ready_pose_detected: last_database
                .majority_vote_is_referee_ready_pose_detected
                .into(),
            visual_referee_proceed_to_ready: last_database.visual_referee_proceed_to_ready.into(),
            hypothetical_ball_positions: last_database.hypothetical_ball_positions.clone().into(),
            is_localization_converged: last_database.is_localization_converged.into(),
            obstacles: last_database.obstacles.clone().into(),
            penalty_shot_direction: last_database.penalty_shot_direction.into(),
            ground_to_field: last_database.ground_to_field.into(),
            sensor_data: last_database.sensor_data.clone().into(),
            stand_up_front_estimated_remaining_duration: last_database
                .stand_up_front_estimated_remaining_duration
                .into(),
            stand_up_back_estimated_remaining_duration: last_database
                .stand_up_back_estimated_remaining_duration
                .into(),
            calibration_command: last_database.calibration_command.into(),
            robot_to_ground: last_database.robot_to_ground.into(),
            robot_orientation: last_database.robot_orientation.into(),
            obstacle_avoiding_arms: last_database.obstacle_avoiding_arms.into(),
            zero_moment_point: last_database.zero_moment_point.into(),
            number_of_consecutive_cycles_zero_moment_point_outside_support_polygon: last_database
                .number_of_consecutive_cycles_zero_moment_point_outside_support_polygon
                .into(),
            support_foot: last_database.support_foot.into(),
        })
    }
}
