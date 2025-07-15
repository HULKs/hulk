use std::{
    convert::Into,
    f32::consts::FRAC_PI_2,
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

use buffered_watch::{Receiver, Sender};
use control::{localization::generate_initial_pose, zero_moment_point_provider::LEFT_FOOT_OUTLINE};
use coordinate_systems::{Field, Ground, Head, LeftSole, RightSole, Robot as RobotCoordinates};
use framework::{future_queue, Producer, RecordingTrigger};
use geometry::{circle::Circle, polygon::circle_overlaps_polygon};
use hula_types::hardware::Ids;
use linear_algebra::{
    point, vector, Isometry2, Isometry3, Orientation2, Orientation3, Point2, Pose2, Pose3,
    Rotation2, Vector2,
};
use parameters::directory::deserialize;
use projection::intrinsic::Intrinsic;
use spl_network_messages::{HulkMessage, PlayerNumber};
use types::{
    ball_position::BallPosition,
    filtered_whistle::FilteredWhistle,
    joints::Joints,
    messages::{IncomingMessage, OutgoingMessage},
    motion_command::{HeadMotion, KickVariant},
    motion_selection::MotionSafeExits,
    pose_kinds::PoseKind,
    robot_dimensions::RobotDimensions,
    sensor_data::Foot,
    support_foot::Side,
};
use walking_engine::{
    kick_state::KickState,
    mode::{kicking::Kicking, Mode},
};

use crate::{
    ball::BallResource,
    cyclers::control::{Cycler, CyclerInstance, Database},
    game_controller::GameController,
    interfake::{FakeDataInterface, Interfake},
    structs::Parameters,
    visual_referee::VisualRefereeResource,
    whistle::WhistleResource,
};

#[derive(Component)]
pub struct Robot {
    pub interface: Arc<Interfake>,
    pub database: Database,
    pub parameters: Parameters,
    pub last_kick_time: Duration,
    pub simulator_parameters: SimulatedRobotParameters,
    pub anchor: Pose2<Field>,
    pub anchor_side: Option<Side>,

    pub cycler: Cycler<Interfake>,
    control_receiver: Receiver<(SystemTime, Database)>,
    parameters_sender: Sender<(SystemTime, Parameters)>,
    spl_network_sender: Producer<crate::structs::spl_network::MainOutputs>,
    object_detection_top_sender: Producer<crate::structs::object_detection::MainOutputs>,
}

impl Robot {
    pub fn new(player_number: PlayerNumber) -> Self {
        Self::try_new(player_number).expect("failed to create robot")
    }

    pub fn try_new(player_number: PlayerNumber) -> Result<Self> {
        let mut parameters: Parameters = deserialize(
            "etc/parameters",
            &Ids {
                body_id: format!("behavior_simulator.{}", from_player_number(player_number)),
                head_id: format!("behavior_simulator.{}", from_player_number(player_number)),
            },
            true,
        )
        .wrap_err("could not load initial parameters")?;
        parameters.player_number = player_number;

        let interface: Arc<_> = Interfake::default().into();

        let (control_sender, control_receiver) =
            buffered_watch::channel((UNIX_EPOCH, Database::default()));
        let (mut subscriptions_sender, subscriptions_receiver) =
            buffered_watch::channel(Default::default());
        let (mut parameters_sender, parameters_receiver) =
            buffered_watch::channel((UNIX_EPOCH, Default::default()));
        let (spl_network_sender, spl_network_consumer) = future_queue();
        let (recording_sender, _recording_receiver) = mpsc::sync_channel(0);
        let (object_detection_top_sender, object_detection_top_consumer) = future_queue();

        *parameters_sender.borrow_mut() = (SystemTime::now(), parameters.clone());

        let mut cycler = Cycler::new(
            CyclerInstance::Control,
            interface.clone(),
            control_sender,
            subscriptions_receiver,
            parameters_receiver,
            spl_network_consumer,
            object_detection_top_consumer,
            recording_sender,
            RecordingTrigger::new(0),
        )?;
        cycler.cycler_state.motion_safe_exits = MotionSafeExits::fill(true);

        let mut database = Database::default();

        database.main_outputs.ground_to_field = Some(
            generate_initial_pose(
                &parameters.localization.initial_poses[player_number],
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

        let simulator_parameters = SimulatedRobotParameters {
            ball_view_range: 3.0,
            ball_timeout_factor: 0.1,
        };

        Ok(Self {
            interface,
            database,
            parameters,
            last_kick_time: Duration::default(),
            simulator_parameters,
            anchor: Pose2::zero(),
            anchor_side: None,

            cycler,
            control_receiver,
            parameters_sender,
            spl_network_sender,
            object_detection_top_sender,
        })
    }

    pub fn cycle(
        &mut self,
        messages: &[Message],
        referee_pose_kind: &Option<PoseKind>,
    ) -> Result<()> {
        for Message { sender, payload } in messages {
            let source_is_other = *sender != self.parameters.player_number;
            let message = IncomingMessage::Spl(*payload);
            self.spl_network_sender.announce();
            self.spl_network_sender
                .finalize(crate::structs::spl_network::MainOutputs {
                    filtered_message: source_is_other.then(|| message.clone()),
                    message,
                });
        }

        self.object_detection_top_sender.announce();
        self.object_detection_top_sender
            .finalize(crate::structs::object_detection::MainOutputs {
                referee_pose_kind: referee_pose_kind.clone(),
                ..Default::default()
            });

        buffered_watch::Sender::<_>::borrow_mut(
            &mut self.interface.get_last_database_sender().lock(),
        )
        .main_outputs = self.database.main_outputs.clone();
        *self.parameters_sender.borrow_mut() = (SystemTime::now(), self.parameters.clone());

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
        let field_of_view = Intrinsic::calculate_field_of_view(focal_lengths_scaled, image_size);

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

pub fn to_player_number(value: usize) -> Result<PlayerNumber, String> {
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

pub fn from_player_number(val: PlayerNumber) -> usize {
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
        if let Some(ball) = robot.database.main_outputs.ball_position.as_mut() {
            ball.position += ball.velocity * time.delta_secs();
            ball.velocity *= 0.98
        }
        if let Mode::Kicking(Kicking {
            kick:
                KickState {
                    variant,
                    side: kicking_side,
                    strength,
                    ..
                },
            ..
        }) = robot.cycler.cycler_state.walking_engine_mode
        {
            if let Some(ball) = ball.state.as_mut() {
                let side = match kicking_side {
                    Side::Left => -1.0,
                    Side::Right => 1.0,
                };

                let robot_to_ground = robot.database.main_outputs.robot_to_ground.unwrap();
                let kinematics = &robot.database.main_outputs.robot_kinematics;
                let left_sole_to_ground = robot_to_ground * kinematics.left_leg.sole_to_robot;
                let right_sole_to_ground = robot_to_ground * kinematics.right_leg.sole_to_robot;
                let left_sole_in_ground: Vec<_> = LEFT_FOOT_OUTLINE
                    .into_iter()
                    .map(|point| (left_sole_to_ground * point).xy())
                    .collect();
                let right_sole_in_ground: Vec<_> = LEFT_FOOT_OUTLINE
                    .into_iter()
                    .map(|point| {
                        (right_sole_to_ground * point![point.x(), -point.y(), point.z()]).xy()
                    })
                    .collect();

                let ball_circle =
                    Circle::new(robot.ground_to_field().inverse() * ball.position, 0.05);
                let in_range = circle_overlaps_polygon(&left_sole_in_ground, ball_circle)
                    || circle_overlaps_polygon(&right_sole_in_ground, ball_circle);
                let previous_kick_finished =
                    (time.elapsed() - robot.last_kick_time).as_secs_f32() > 1.0;
                if in_range && previous_kick_finished {
                    let direction = match variant {
                        KickVariant::Forward => Orientation2::identity(),
                        KickVariant::Turn => Orientation2::new(0.35),
                        KickVariant::Side => Orientation2::new(-FRAC_PI_2),
                    }
                    .as_unit_vector()
                    .component_mul(&vector![1.0, side]);
                    ball.velocity += robot.ground_to_field() * direction * strength * 2.5;
                    robot.last_kick_time = time.elapsed();
                };
            }
        };

        let (left_sole, right_sole) =
            sole_positions(&robot.database.main_outputs.sensor_data.positions);
        let support_foot = robot
            .database
            .main_outputs
            .support_foot
            .support_side
            .unwrap();
        if robot.anchor_side != Some(support_foot) {
            robot.anchor_side = Some(support_foot);
            let support_sole = match support_foot {
                Side::Left => left_sole,
                Side::Right => right_sole,
            };
            let ground = robot.database.main_outputs.robot_to_ground.unwrap() * support_sole;
            robot.anchor = robot.ground_to_field() * to2d(ground);
        }

        let target = robot.database.main_outputs.walk_motor_commands.positions;
        let positions = &mut robot.database.main_outputs.sensor_data.positions;
        positions.left_leg =
            positions.left_leg + (target.left_leg - positions.left_leg) * time.delta_secs() * 10.0;
        positions.right_leg = positions.right_leg
            + (target.right_leg - positions.right_leg) * time.delta_secs() * 10.0;
        positions.left_arm =
            positions.left_arm + (target.left_arm - positions.left_arm) * time.delta_secs() * 10.0;
        positions.right_arm = positions.right_arm
            + (target.right_arm - positions.right_arm) * time.delta_secs() * 10.0;

        let (new_left_sole, new_right_sole) =
            sole_positions(&robot.database.main_outputs.sensor_data.positions);
        let support_sole = match support_foot {
            Side::Left => new_left_sole,
            Side::Right => new_right_sole,
        };
        let ground = robot.database.main_outputs.robot_to_ground.unwrap() * support_sole;
        let new_anchor = robot.ground_to_field() * to2d(ground);
        let movement = robot.anchor.as_transform() * new_anchor.as_transform::<Field>().inverse();
        let step = robot.ground_to_field().inverse() * movement * robot.ground_to_field();
        let ground_to_field_change = Some(Isometry2::from_parts(
            step.translation().coords(),
            step.orientation().angle(),
        ));

        let head_motion = robot
            .database
            .main_outputs
            .motion_command
            .head_motion()
            .unwrap_or(HeadMotion::Center);
        let desired_head_yaw = match head_motion {
            HeadMotion::ZeroAngles => 0.0,
            HeadMotion::Center => 0.0,
            HeadMotion::LookAround | HeadMotion::SearchForLostBall => {
                robot.database.main_outputs.look_around.yaw
            }
            HeadMotion::LookAt { target, .. } => Orientation2::from_vector(target.coords()).angle(),
            HeadMotion::LookAtReferee { .. } => {
                if let Some(ground_to_field) = robot.database.main_outputs.ground_to_field {
                    let expected_referee_position = ground_to_field.inverse()
                        * robot
                            .database
                            .main_outputs
                            .expected_referee_position
                            .unwrap_or_default();
                    Orientation2::from_vector(expected_referee_position.coords()).angle()
                } else {
                    0.0
                }
            }
            HeadMotion::LookLeftAndRightOf { target } => {
                let glance_factor = time.elapsed().as_secs_f32().sin();
                target.coords().angle(&Vector2::x_axis())
                    + glance_factor * robot.parameters.look_at.glance_angle
            }
            HeadMotion::Unstiff => 0.0,
            HeadMotion::Animation { .. } => 0.0,
        };

        let max_head_rotation_per_cycle =
            robot.parameters.head_motion.maximum_velocity.yaw * time.delta_secs();
        let diff = desired_head_yaw - robot.database.main_outputs.sensor_data.positions.head.yaw;
        let movement = diff.clamp(-max_head_rotation_per_cycle, max_head_rotation_per_cycle);
        robot.database.main_outputs.sensor_data.positions.head.yaw += movement;

        if let Some(movement) = ground_to_field_change {
            let old_ground_to_field = robot.ground_to_field();
            let new_ground_to_field = old_ground_to_field * movement;

            for obstacle in &mut robot.database.main_outputs.obstacles {
                let obstacle_in_field = old_ground_to_field * obstacle.position;
                obstacle.position = new_ground_to_field.inverse() * obstacle_in_field;
            }
            if let Some(ball) = robot.database.main_outputs.ball_position.as_mut() {
                ball.velocity = movement.inverse() * ball.velocity;
                ball.position = movement.inverse() * ball.position;
            }

            *robot.ground_to_field_mut() = new_ground_to_field;
        }
    }
}

#[derive(Event, Clone, Copy)]
pub struct Message {
    pub sender: PlayerNumber,
    pub payload: HulkMessage,
}

#[derive(Resource, Default)]
pub struct Messages {
    pub messages: Vec<Message>,
}

#[allow(clippy::too_many_arguments)]
pub fn cycle_robots(
    mut robots: Query<&mut Robot>,
    ball: Res<BallResource>,
    whistle: Res<WhistleResource>,
    visual_referee: Res<VisualRefereeResource>,
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
            let angle_to_ball = ball_in_head.coords().angle(&Vector2::x_axis());

            angle_to_ball.abs() < field_of_view / 2.0
                && ball_in_head.coords().norm() < robot.simulator_parameters.ball_view_range
        });
        if ball_visible {
            robot.database.main_outputs.ball_position =
                ball.state.as_ref().map(|ball| BallPosition {
                    position: robot.ground_to_field().inverse() * ball.position,
                    velocity: robot.ground_to_field().inverse() * ball.velocity,
                    last_seen: now,
                });
        }
        if !robot
            .database
            .main_outputs
            .ball_position
            .is_some_and(|ball_position| {
                now.duration_since(ball_position.last_seen)
                    .expect("time ran backwards")
                    < robot
                        .parameters
                        .ball_filter
                        .hypothesis_timeout
                        .mul_f32(robot.simulator_parameters.ball_timeout_factor)
            })
        {
            robot.database.main_outputs.ball_position = None
        };
        *robot.whistle_mut() = FilteredWhistle {
            is_detected: Some(time.elapsed()) == whistle.last_whistle,
            last_detection: whistle
                .last_whistle
                .map(|last_whistle| SystemTime::UNIX_EPOCH + last_whistle),
        };
        let visual_referee_pose_kind = if matches!(
            robot.database.main_outputs.motion_command.head_motion(),
            Some(HeadMotion::LookAtReferee { .. })
        ) {
            visual_referee.pose_kind.clone()
        } else {
            None
        };
        robot.database.main_outputs.game_controller_state = Some(game_controller.state.clone());
        robot.cycler.cycler_state.ground_to_field = robot.ground_to_field();
        robot.interface.set_time(now);
        robot.database.main_outputs.robot_orientation = robot
            .database
            .main_outputs
            .robot_orientation
            .or(Some(Orientation3::default()));
        robot
            .cycle(&messages_sent_last_cycle, &visual_referee_pose_kind)
            .unwrap();

        // Walking physics
        let support_foot = robot
            .database
            .main_outputs
            .support_foot
            .support_side
            .unwrap();
        let is_step_finished = robot
            .cycler
            .cycler_state
            .walking_engine_mode
            .step_state()
            .is_some_and(|step_state| step_state.time_since_start >= step_state.plan.step_duration);
        let next_support_foot = if is_step_finished {
            support_foot.opposite()
        } else {
            support_foot
        };
        let (left_pressure, right_pressure) = match next_support_foot {
            Side::Left => (1.0, 0.0),
            Side::Right => (0.0, 1.0),
        };

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
                    sender: robot.parameters.player_number,
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

pub struct SimulatedRobotParameters {
    pub ball_view_range: f32,
    pub ball_timeout_factor: f32,
}

fn sole_positions(joint_positions: &Joints) -> (Pose3<RobotCoordinates>, Pose3<RobotCoordinates>) {
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

    (left_sole_to_robot.as_pose(), right_sole_to_robot.as_pose())
}

fn to2d<To>(iso: Pose3<To>) -> Pose2<To> {
    Pose2::from_parts(
        iso.position().xy(),
        Orientation2::new(iso.orientation().inner.euler_angles().2),
    )
}
