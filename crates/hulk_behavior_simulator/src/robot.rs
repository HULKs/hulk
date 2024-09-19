use std::{
    convert::Into,
    mem::take,
    sync::{mpsc, Arc},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use bevy::{
    ecs::{
        component::Component,
        event::Event,
        system::{Query, Res, ResMut, Resource},
    },
    time::Time,
};
use color_eyre::{eyre::WrapErr, Result};

use buffered_watch::Receiver;
use control::localization::generate_initial_pose;
use coordinate_systems::{Field, Ground, Head, LeftSole, RightSole, Robot as RobotCoordinates};
use framework::{future_queue, Producer, RecordingTrigger};
use geometry::line_segment::LineSegment;
use linear_algebra::{
    vector, Isometry2, Isometry3, Orientation2, Orientation3, Point2, Rotation2, Vector2,
};
use parameters::directory::deserialize;
use projection::camera_matrix::CameraMatrix;
use spl_network_messages::{HulkMessage, PlayerNumber};
use types::{
    ball_position::BallPosition,
    filtered_whistle::FilteredWhistle,
    hardware::Ids,
    joints::Joints,
    messages::{IncomingMessage, OutgoingMessage},
    motion_command::{HeadMotion, KickVariant, MotionCommand, OrientationMode},
    motion_selection::MotionSafeExits,
    planned_path::PathSegment,
    robot_dimensions::RobotDimensions,
    sensor_data::Foot,
    support_foot::Side,
};

use crate::{
    ball::BallResource,
    cyclers::control::{Cycler, CyclerInstance, Database},
    game_controller::GameController,
    interfake::{FakeDataInterface, Interfake},
    structs::Parameters,
    whistle::WhistleResource,
};

#[derive(Component)]
pub struct Robot {
    pub interface: Arc<Interfake>,
    pub database: Database,
    pub parameters: Parameters,
    pub last_kick_time: Duration,
    pub ball_last_seen: Option<SystemTime>,

    pub cycler: Cycler<Interfake>,
    control_receiver: Receiver<(SystemTime, Database)>,
    spl_network_sender: Producer<crate::structs::spl_network::MainOutputs>,
}

impl Robot {
    pub fn new(jersey_number: usize) -> Self {
        Self::try_new(jersey_number).expect("failed to create robot")
    }

    pub fn try_new(jersey_number: usize) -> Result<Self> {
        let mut parameters: Parameters = deserialize(
            "etc/parameters",
            &Ids {
                body_id: format!("behavior_simulator.{}", jersey_number),
                head_id: format!("behavior_simulator.{}", jersey_number),
            },
            true,
        )
        .wrap_err("could not load initial parameters")?;
        parameters.jersey_number = jersey_number;

        let interface: Arc<_> = Interfake::default().into();

        let (control_sender, control_receiver) =
            buffered_watch::channel((UNIX_EPOCH, Database::default()));
        let (mut subscriptions_sender, subscriptions_receiver) =
            buffered_watch::channel(Default::default());
        let (mut parameters_sender, parameters_receiver) =
            buffered_watch::channel((UNIX_EPOCH, Default::default()));
        let (spl_network_sender, spl_network_consumer) = future_queue();
        let (recording_sender, _recording_receiver) = mpsc::sync_channel(0);
        *parameters_sender.borrow_mut() = (SystemTime::now(), parameters.clone());

        let mut cycler = Cycler::new(
            CyclerInstance::Control,
            interface.clone(),
            control_sender,
            subscriptions_receiver,
            parameters_receiver,
            spl_network_consumer,
            recording_sender,
            RecordingTrigger::new(0),
        )?;
        cycler.cycler_state.motion_safe_exits = MotionSafeExits::fill(true);

        let mut database = Database::default();

        database.main_outputs.ground_to_field = Some(
            generate_initial_pose(
                &parameters.localization.initial_poses[jersey_number-1],
                &parameters.field_dimensions,
            )
            .as_transform(),
        );
        database.main_outputs.has_ground_contact = true;
        database.main_outputs.buttons.is_chest_button_pressed_once = true;
        database.main_outputs.is_localization_converged = true;

        subscriptions_sender
            .borrow_mut()
            .insert("additional_outputs".to_string());

        Ok(Self {
            interface,
            database,
            parameters,
            last_kick_time: Duration::default(),
            ball_last_seen: None,

            cycler,
            control_receiver,
            spl_network_sender,
        })
    }

    pub fn cycle(&mut self, messages: &[Message]) -> Result<()> {
        for Message { sender, payload } in messages {
            let source_is_other = *sender != self.parameters.jersey_number;
            let message = IncomingMessage::Spl(*payload);
            self.spl_network_sender.announce();
            self.spl_network_sender
                .finalize(crate::structs::spl_network::MainOutputs {
                    filtered_message: source_is_other.then(|| message.clone()),
                    message,
                });
        }
        buffered_watch::Sender::<_>::borrow_mut(
            &mut self.interface.get_last_database_sender().lock(),
        )
        .main_outputs = self.database.main_outputs.clone();

        self.cycler.cycle()?;

        let (_, database) = &*self.control_receiver.borrow_and_mark_as_seen();
        self.database.main_outputs = database.main_outputs.clone();
        self.database.additional_outputs = database.additional_outputs.clone();
        Ok(())
    }

    pub fn field_of_view(&self) -> f32 {
        let image_size = vector![640.0, 480.0];
        let focal_lengths = self
            .parameters
            .camera_matrix_parameters
            .vision_top
            .focal_lengths;
        let focal_lengths_scaled = image_size.inner.cast().component_mul(&focal_lengths);
        let field_of_view = CameraMatrix::calculate_field_of_view(focal_lengths_scaled, image_size);

        field_of_view.x
    }

    pub fn ground_to_field(&self) -> Isometry2<Ground, Field> {
        self.database
            .main_outputs
            .ground_to_field
            .expect("simulated robots should always have a ground to field")
    }

    pub fn ground_to_field_mut(&mut self) -> &mut Isometry2<Ground, Field> {
        self.database
            .main_outputs
            .ground_to_field
            .as_mut()
            .expect("simulated robots should always have a ground to field")
    }

    pub fn whistle_mut(&mut self) -> &mut FilteredWhistle {
        &mut self.database.main_outputs.filtered_whistle
    }
}

pub fn to_jersey_number(value: usize) -> Result<PlayerNumber, String> {
    let number = match value {
        1 => PlayerNumber::One,
        2 => PlayerNumber::Two,
        3 => PlayerNumber::Three,
        4 => PlayerNumber::Four,
        5 => PlayerNumber::Five,
        6 => PlayerNumber::Six,
        7 => PlayerNumber::Seven,
        number => return Err(format!("invalid player number: {number}")),
    };

    Ok(number)
}

pub fn from_jersey_number(val: PlayerNumber) -> usize {
    match val {
        PlayerNumber::One => 1,
        PlayerNumber::Two => 2,
        PlayerNumber::Three => 3,
        PlayerNumber::Four => 4,
        PlayerNumber::Five => 5,
        PlayerNumber::Six => 6,
        PlayerNumber::Seven => 7,
    }
}

pub fn move_robots(mut robots: Query<&mut Robot>, mut ball: ResMut<BallResource>, time: Res<Time>) {
    for mut robot in &mut robots {
        let parameters = &robot.parameters;
        let mut ground_to_field_update: Option<Isometry2<Ground, Field>> = None;

        let head_motion = match robot.database.main_outputs.motion_command.clone() {
            MotionCommand::Walk {
                head,
                path,
                orientation_mode,
                ..
            } => {
                let steps_per_second = 1.0 / 0.35;
                let steps_this_cycle = steps_per_second * time.delta_seconds();
                let max_step = parameters.step_planner.max_step_size;

                let target = match path[0] {
                    PathSegment::LineSegment(LineSegment(_start, end)) => end.coords(),
                    PathSegment::Arc(arc, direction) => {
                        direction.rotate_vector_90_degrees(arc.start - arc.circle.center)
                    }
                };

                let orientation = match orientation_mode {
                    OrientationMode::AlignWithPath => {
                        if target.norm_squared() < f32::EPSILON {
                            Orientation2::identity()
                        } else {
                            Orientation2::from_vector(target)
                        }
                    }
                    OrientationMode::Override(orientation) => orientation,
                };
                let step = target.cap_magnitude(max_step.forward * steps_this_cycle);

                let rotation = orientation.angle().clamp(
                    -max_step.turn * steps_this_cycle,
                    max_step.turn * steps_this_cycle,
                );
                let movement = Isometry2::from_parts(step.as_point().coords(), rotation);
                let old_ground_to_field = robot.ground_to_field();
                let new_ground_to_field = old_ground_to_field * movement;
                ground_to_field_update = Some(new_ground_to_field);

                for obstacle in &mut robot.database.main_outputs.obstacles {
                    let obstacle_in_field = old_ground_to_field * obstacle.position;
                    obstacle.position = new_ground_to_field.inverse() * obstacle_in_field;
                }

                head
            }
            MotionCommand::InWalkKick {
                head,
                kick,
                kicking_side,
                strength,
                ..
            } => {
                if let Some(ball) = ball.state.as_mut() {
                    let side = match kicking_side {
                        Side::Left => -1.0,
                        Side::Right => 1.0,
                    };

                    // TODO: Check if ball is even in range
                    // let kick_location = ground_to_field * ();
                    if (time.elapsed() - robot.last_kick_time).as_secs_f32() > 1.0 {
                        let direction = match kick {
                            KickVariant::Forward => vector![1.0, 0.0],
                            KickVariant::Turn => vector![0.707, 0.707 * side],
                            KickVariant::Side => vector![0.0, 1.0 * -side],
                        };
                        ball.velocity += robot.ground_to_field() * direction * strength * 2.5;
                        robot.last_kick_time = time.elapsed();
                    };
                }
                head
            }
            MotionCommand::SitDown { head } => head,
            MotionCommand::Stand { head } => head,
            _ => HeadMotion::Center,
        };

        let desired_head_yaw = match head_motion {
            HeadMotion::ZeroAngles => 0.0,
            HeadMotion::Center => 0.0,
            HeadMotion::LookAround | HeadMotion::SearchForLostBall => {
                robot.database.main_outputs.look_around.yaw
            }
            HeadMotion::LookAt { target, .. } => Orientation2::from_vector(target.coords()).angle(),
            HeadMotion::LookLeftAndRightOf { target } => {
                let glance_factor = 0.0; //self.time_elapsed.as_secs_f32().sin();
                target.coords().angle(Vector2::x_axis())
                    + glance_factor * robot.parameters.look_at.glance_angle
            }
            HeadMotion::Unstiff => 0.0,
            HeadMotion::Animation { .. } => 0.0,
        };

        let max_head_rotation_per_cycle =
            robot.parameters.head_motion.maximum_velocity.yaw * time.delta_seconds();
        let diff = desired_head_yaw - robot.database.main_outputs.sensor_data.positions.head.yaw;
        let movement = diff.clamp(-max_head_rotation_per_cycle, max_head_rotation_per_cycle);

        robot.database.main_outputs.sensor_data.positions.head.yaw += movement;
        if let Some(new_ground_to_field) = ground_to_field_update {
            *robot.ground_to_field_mut() = new_ground_to_field;
            let orientation = new_ground_to_field.as_pose().orientation();
            robot.database.main_outputs.robot_orientation = Some(Orientation3::from_euler_angles(
                0.0,
                0.0,
                orientation.angle(),
            ));
        }
    }
}

#[derive(Event, Clone, Copy)]
pub struct Message {
    pub sender: usize,
    pub payload: HulkMessage,
}

#[derive(Resource, Default)]
pub struct Messages {
    pub messages: Vec<Message>,
}

pub fn cycle_robots(
    mut robots: Query<&mut Robot>,
    ball: Res<BallResource>,
    whistle: Res<WhistleResource>,
    mut game_controller: ResMut<GameController>,
    time: Res<Time>,
    mut messages: ResMut<Messages>,
) {
    let messages_sent_last_cycle = take(&mut messages.messages);
    let now = SystemTime::UNIX_EPOCH + time.elapsed();

    for mut robot in &mut robots {
        robot.database.main_outputs.cycle_time.start_time = now;
        robot.database.main_outputs.cycle_time.last_cycle_duration = time.delta();

        let ball_visible = ball.state.as_ref().is_some_and(|ball| {
            let ball_in_ground = robot.ground_to_field().inverse() * ball.position;
            let head_to_ground =
                Rotation2::new(robot.database.main_outputs.sensor_data.positions.head.yaw);
            let ball_in_head: Point2<Head> = head_to_ground.inverse() * ball_in_ground;
            let field_of_view = robot.field_of_view();
            let angle_to_ball = ball_in_head.coords().angle(Vector2::x_axis());

            angle_to_ball.abs() < field_of_view / 2.0 && ball_in_head.coords().norm() < 3.0
        });
        if ball_visible {
            robot.ball_last_seen = Some(now);
        }
        robot.database.main_outputs.ball_position =
            if robot.ball_last_seen.is_some_and(|last_seen| {
                now.duration_since(last_seen).expect("time ran backwards")
                    < robot.parameters.ball_filter.hypothesis_timeout
            }) {
                ball.state.as_ref().map(|ball| BallPosition {
                    position: robot.ground_to_field().inverse() * ball.position,
                    velocity: robot.ground_to_field().inverse() * ball.velocity,
                    last_seen: now,
                })
            } else {
                None
            };
        *robot.whistle_mut() = FilteredWhistle {
            is_detected: Some(time.elapsed()) == whistle.last_whistle,
            last_detection: whistle
                .last_whistle
                .map(|last_whistle| SystemTime::UNIX_EPOCH + last_whistle),
        };
        robot.database.main_outputs.game_controller_state = Some(game_controller.state.clone());
        robot.cycler.cycler_state.ground_to_field = robot.ground_to_field();
        robot.cycle(&messages_sent_last_cycle).unwrap();

        // Walking physics
        let target = robot.database.main_outputs.walk_motor_commands.positions;
        robot.database.main_outputs.sensor_data.positions.left_leg =
            robot.database.main_outputs.sensor_data.positions.left_leg
                + (target.left_leg - robot.database.main_outputs.sensor_data.positions.left_leg)
                    * time.delta_seconds()
                    * 10.0;
        robot.database.main_outputs.sensor_data.positions.right_leg =
            robot.database.main_outputs.sensor_data.positions.right_leg
                + (target.right_leg - robot.database.main_outputs.sensor_data.positions.right_leg)
                    * time.delta_seconds()
                    * 10.0;
        let way_to_go = (robot.database.main_outputs.sensor_data.positions.left_leg
            - target.left_leg)
            .into_iter()
            .map(|x| x.abs())
            .sum::<f32>()
            + (robot.database.main_outputs.sensor_data.positions.right_leg - target.right_leg)
                .into_iter()
                .map(|x| x.abs())
                .sum::<f32>();

        let (left_sole_height, right_sole_height) =
            sole_heights(&robot.database.main_outputs.sensor_data.positions);
        let floor_height = right_sole_height.min(left_sole_height);
        let grass_height = 0.001;

        let support_foot = robot
            .database
            .main_outputs
            .support_foot
            .support_side
            .unwrap();
        let step_ended = way_to_go < 0.01;
        let (left_step_end_bonus, right_step_end_bonus) = match (step_ended, support_foot) {
            (false, _) => (0.0, 0.0),
            (true, Side::Left) => (0.0, 0.5),
            (true, Side::Right) => (0.5, 0.0),
        };

        let left_sink = sole_sink(
            left_sole_height,
            floor_height,
            grass_height,
            left_step_end_bonus,
        );
        let right_sink = sole_sink(
            right_sole_height,
            floor_height,
            grass_height,
            right_step_end_bonus,
        );
        let left_pressure = left_sink / (left_sink + right_sink) * 3.0;
        let right_pressure = right_sink / (left_sink + right_sink) * 3.0;
        robot
            .database
            .main_outputs
            .sensor_data
            .force_sensitive_resistors
            .left = Foot::fill(left_pressure);
        robot
            .database
            .main_outputs
            .sensor_data
            .force_sensitive_resistors
            .right = Foot::fill(right_pressure);

        for message in robot.interface.take_outgoing_messages() {
            if let OutgoingMessage::Spl(message) = message {
                messages.messages.push(Message {
                    sender: robot.parameters.jersey_number,
                    payload: message,
                });
                game_controller
                    .state
                    .hulks_team
                    .remaining_amount_of_messages -= 1
            }
        }
    }
}

fn sole_sink(sole_height: f32, floor_height: f32, grass_height: f32, step_end_bonus: f32) -> f32 {
    (sole_height - floor_height - grass_height).clamp(-grass_height, 0.0) + step_end_bonus
}

fn sole_heights(joint_positions: &Joints) -> (f32, f32) {
    use kinematics::forward::*;
    // left leg
    let left_pelvis_to_robot = left_pelvis_to_robot(&joint_positions.left_leg);
    let left_hip_to_robot =
        left_pelvis_to_robot * left_hip_to_left_pelvis(&joint_positions.left_leg);
    let left_thigh_to_robot = left_hip_to_robot * left_thigh_to_left_hip(&joint_positions.left_leg);
    let left_tibia_to_robot =
        left_thigh_to_robot * left_tibia_to_left_thigh(&joint_positions.left_leg);
    let left_ankle_to_robot =
        left_tibia_to_robot * left_ankle_to_left_tibia(&joint_positions.left_leg);
    let left_foot_to_robot =
        left_ankle_to_robot * left_foot_to_left_ankle(&joint_positions.left_leg);
    let left_sole_to_robot: Isometry3<LeftSole, RobotCoordinates> =
        left_foot_to_robot * Isometry3::from(RobotDimensions::LEFT_ANKLE_TO_LEFT_SOLE);
    // right leg
    let right_pelvis_to_robot = right_pelvis_to_robot(&joint_positions.right_leg);
    let right_hip_to_robot =
        right_pelvis_to_robot * right_hip_to_right_pelvis(&joint_positions.right_leg);
    let right_thigh_to_robot =
        right_hip_to_robot * right_thigh_to_right_hip(&joint_positions.right_leg);
    let right_tibia_to_robot =
        right_thigh_to_robot * right_tibia_to_right_thigh(&joint_positions.right_leg);
    let right_ankle_to_robot =
        right_tibia_to_robot * right_ankle_to_right_tibia(&joint_positions.right_leg);
    let right_foot_to_robot =
        right_ankle_to_robot * right_foot_to_right_ankle(&joint_positions.right_leg);
    let right_sole_to_robot: Isometry3<RightSole, RobotCoordinates> =
        right_foot_to_robot * Isometry3::from(RobotDimensions::RIGHT_ANKLE_TO_RIGHT_SOLE);

    (
        left_sole_to_robot.as_pose().position().z(),
        right_sole_to_robot.as_pose().position().z(),
    )
}
