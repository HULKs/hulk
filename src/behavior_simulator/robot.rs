use std::{
    convert::TryFrom,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Context};
use nalgebra::{distance, Isometry2, Point2, Rotation2, Vector2};
use serde::Serialize;
use spl_network::{GameState, SplMessage};

use crate::{
    control::{generate_initial_isometry2, BehaviorCycler, Database},
    framework::Configuration,
    types::{BallPosition, CycleInfo, FallState, Motion, PrimaryState, SensorData},
};

use super::state::State;

#[derive(Serialize)]
pub struct Robot {
    pub configuration: Configuration,
    #[serde(skip)]
    pub cycler: BehaviorCycler,
    pub is_penalized: bool,
    pub robot_to_field: Isometry2<f32>,
    #[serde(skip)]
    last_step: SystemTime,
    #[serde(skip)]
    next_action: NextAction,
}

enum NextAction {
    DoNothing,
    WalkTo { end_pose: Isometry2<f32> },
}

impl TryFrom<Configuration> for Robot {
    type Error = anyhow::Error;

    fn try_from(configuration: Configuration) -> anyhow::Result<Self> {
        let cycler =
            BehaviorCycler::new(&configuration).context("Failed to construct BahaviorCycler")?;
        let initial_pose =
            &configuration.control.pose_estimation.initial_poses[configuration.player_number];
        let initial_pose =
            generate_initial_isometry2(initial_pose, &configuration.field_dimensions);
        Ok(Self {
            configuration,
            cycler,
            is_penalized: false,
            robot_to_field: initial_pose,
            last_step: UNIX_EPOCH,
            next_action: NextAction::DoNothing,
        })
    }
}

impl Robot {
    pub fn step(
        &mut self,
        state: &State,
    ) -> anyhow::Result<(Database, Vec<SplMessage>, Option<Vector2<f32>>)> {
        self.apply_action(state);

        let (sensor_data, ball_position, fall_state, primary_state) = self.inputs_from_state(state);

        let database = self
            .cycler
            .run_cycle(
                &self.configuration,
                state.now,
                ball_position,
                fall_state,
                self.robot_to_field,
                sensor_data,
                primary_state,
                state.broadcasted_spl_messages.clone(),
            )
            .context("Failed to run cycle")?;

        self.apply_outputs_and_set_next_action(&database)
            .context("Failed to apply outputs")?;

        self.extract_state_modifications(state, database)
            .context("Failed to extract state modifications")
    }

    fn apply_action(&mut self, state: &State) {
        match self.next_action {
            NextAction::DoNothing => {}
            NextAction::WalkTo { end_pose } => {
                let end_pose_in_field = self.robot_to_field * end_pose;
                let angle_difference = end_pose.rotation.angle();
                let translation_difference =
                    end_pose_in_field.translation.vector - self.robot_to_field.translation.vector;
                let translation_difference_distance = translation_difference.norm();
                let angle = self.robot_to_field.rotation.angle()
                    + angle_difference.signum()
                        * f32::min(
                            state.configuration.maximum_walk_angle_per_second
                                * state.configuration.time_step.as_secs_f32(),
                            angle_difference.abs(),
                        );
                let translation = if translation_difference_distance == 0.0 {
                    Vector2::zeros()
                } else {
                    translation_difference.normalize()
                        * f32::min(
                            state
                                .configuration
                                .maximum_walk_translation_distance_per_second
                                * state.configuration.time_step.as_secs_f32(),
                            translation_difference_distance,
                        )
                };
                self.robot_to_field =
                    Isometry2::new(self.robot_to_field.translation.vector + translation, angle);
            }
        }
    }

    fn inputs_from_state(
        &mut self,
        state: &State,
    ) -> (SensorData, BallPosition, FallState, PrimaryState) {
        let cycle_info = CycleInfo {
            start_time: state.now,
            last_cycle_duration: state
                .now
                .duration_since(self.last_step)
                .expect("Time ran backwards"),
        };
        self.last_step = state.now;

        let sensor_data = SensorData {
            cycle_info,
            positions: Default::default(),
            inertial_measurement_unit: Default::default(),
            sonar_sensors: Default::default(),
            force_sensitive_resistors: Default::default(),
            touch_sensors: Default::default(),
        };

        let ball_position = BallPosition {
            position: limit_ball_visibility(
                self.robot_to_field.inverse() * state.ball_position,
                state.configuration.maximum_field_of_view_angle,
                state.configuration.maximum_field_of_view_distance,
            ),
            last_seen: state.now,
        };

        let fall_state = FallState::Upright;
        let primary_state = match (self.is_penalized, state.game_state) {
            (true, _) => PrimaryState::Penalized,
            (false, GameState::Initial) => PrimaryState::Initial,
            (false, GameState::Ready) => PrimaryState::Ready,
            (false, GameState::Set) => PrimaryState::Set,
            (false, GameState::Playing) => PrimaryState::Playing,
            (false, GameState::Finished) => PrimaryState::Finished,
        };

        (sensor_data, ball_position, fall_state, primary_state)
    }

    fn apply_outputs_and_set_next_action(&mut self, database: &Database) -> anyhow::Result<()> {
        let motion_command = database
            .main_outputs
            .motion_command
            .ok_or_else(|| anyhow!("MotionCommand is None"))?;
        self.next_action = match motion_command.motion {
            Motion::FallProtection { .. } => NextAction::DoNothing,
            Motion::Jump { .. } => NextAction::DoNothing,
            Motion::Kick { .. } => NextAction::DoNothing,
            Motion::Penalized => NextAction::DoNothing,
            Motion::SitDown { .. } => NextAction::DoNothing,
            Motion::Stand { .. } => NextAction::DoNothing,
            Motion::StandUp { .. } => NextAction::DoNothing,
            Motion::Unstiff => NextAction::DoNothing,
            Motion::Walk { .. } => {
                let end_pose = database
                    .main_outputs
                    .planned_path
                    .ok_or_else(|| anyhow!("PlannedPath is None"))?
                    .end_pose;
                NextAction::WalkTo { end_pose }
            }
        };

        Ok(())
    }

    fn extract_state_modifications(
        &self,
        state: &State,
        database: Database,
    ) -> anyhow::Result<(Database, Vec<SplMessage>, Option<Vector2<f32>>)> {
        let message_receivers = database
            .main_outputs
            .message_receivers
            .as_ref()
            .ok_or_else(|| anyhow!("MessageReceivers is None"))?;
        {
            let mut game_controller_return_message_receiver = message_receivers
                .game_controller_return_message_receiver
                .blocking_lock();
            while game_controller_return_message_receiver.try_recv().is_ok() {
                // do nothing to drop message
            }
        }
        let spl_messages = {
            let mut spl_message_receiver = message_receivers.spl_message_receiver.blocking_lock();
            let mut spl_messages = vec![];
            while let Ok(spl_message) = spl_message_receiver.try_recv() {
                spl_messages.push(spl_message);
            }
            spl_messages
        };

        let robot_position = self.robot_to_field * Point2::origin();
        let distance = distance(&robot_position, &state.ball_position);
        let ball_bounce_direction =
            if distance > 0.0 && distance <= state.configuration.robot_ball_bounce_radius {
                Some((state.ball_position - robot_position).normalize())
            } else {
                None
            };

        Ok((database, spl_messages, ball_bounce_direction))
    }
}

fn limit_ball_visibility(
    ball_position: Point2<f32>,
    field_of_view_angle_limit: f32,
    field_of_view_distance_limit: f32,
) -> Option<Point2<f32>> {
    if distance(&Point2::origin(), &ball_position) > field_of_view_distance_limit {
        return None;
    }

    let rotation_to_ball = Rotation2::rotation_between(&Vector2::x(), &ball_position.coords);
    if rotation_to_ball.angle().abs() > field_of_view_angle_limit {
        return None;
    }

    Some(ball_position)
}
