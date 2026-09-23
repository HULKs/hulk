use std::{net::SocketAddr, time::Duration};

use behavior_node::{
    behavior_tree::Node as BehaviorNodeTree, motion_assembler::assemble_motion_command,
    node::Blackboard as BehaviorBlackboard, tree::create_tree as create_behavior_tree,
};
use bevy::{app::AppExit, prelude::*};
use color_eyre::Result;
use coordinate_systems::{Field, Ground};
use linear_algebra::{Point2, Pose2, Vector2};
use types::{
    ball_position::BallPosition,
    behavior_tree::NodeTrace,
    field_dimensions::FieldDimensions,
    messages::OutgoingMessage,
    motion_command::MotionCommand,
    parameters::{BehaviorParameters, HslNetworkParameters},
    path_obstacles::PathObstacle,
    world_state::WorldState,
};
use voronoi::VoronoiGrid;

use crate::behavior_tree_simulator::{
    InvariantSeverity, InvariantViolation, RobotFrame, SimulatorClock,
    SimulatorCurrentInvariantViolations, SimulatorFieldDimensions, SimulatorRobot,
    SimulatorRobotFrames, SimulatorRobotParameters, SimulatorScenarioResult, SimulatorWorldStates,
};
use crate::invariant_checks::BEHAVIOR_TICK_ERROR_CHECK_NAME;

#[derive(Component)]
pub struct SimulatorRobotBehavior {
    pub tree: BehaviorNodeTree<BehaviorBlackboard>,
    pub blackboard: BehaviorBlackboard,
    pub static_layout: NodeTrace,
}

impl SimulatorRobotBehavior {
    pub fn new(parameters: BehaviorParameters) -> Self {
        let tree = create_behavior_tree();
        let static_layout = tree.static_layout_trace();
        Self {
            tree,
            blackboard: create_behavior_blackboard(parameters),
            static_layout,
        }
    }

    pub fn tick_behavior_tree(
        &mut self,
        input: SimulatorBehaviorTickInput,
    ) -> Result<SimulatorBehaviorTickOutput> {
        self.blackboard.field_dimensions = input.field_dimensions;
        self.blackboard.parameters = input.parameters;
        self.blackboard.world_state = input.world_state.clone();
        self.blackboard.visual_kick_ball_position = input.visual_kick_ball_position;

        self.blackboard.path_obstacles_output.clear();
        self.blackboard.time_since_last_switch = Duration::ZERO;
        self.blackboard.direction_difference = 0.0;
        self.blackboard.voronoi_inputs.clear();
        self.blackboard.is_injected_motion_command = false;
        self.blackboard.walk_position = None;
        self.blackboard.kick_target = None;
        self.blackboard.body_motion = None;
        self.blackboard.head_motion = None;
        self.blackboard.voronoi_map = None;

        if let Some(ball) = self.blackboard.world_state.ball {
            self.blackboard.ball = Some(behavior_node::node::LastBall {
                position: ball.ball_in_field,
                velocity: ball.ball_in_ground_velocity,
                age: self.blackboard.world_state.now,
                field_side: ball.field_side,
            });
            self.blackboard.last_ball.clone_from(&self.blackboard.ball);
        } else if let Some(last_ball) = &self.blackboard.ball
            && self
                .blackboard
                .world_state
                .now
                .duration_since(last_ball.age)
                >= self.blackboard.parameters.ball.last_ball_timeout
        {
            self.blackboard.ball = None;
        }

        let (status, trace) = self.tree.tick_with_trace(&mut self.blackboard);
        let motion_command = assemble_motion_command(&self.blackboard, status)?;
        self.blackboard.last_motion_command = motion_command.clone();

        let motion_type = match motion_command.clone() {
            MotionCommand::Kick { .. } => Some(types::motion_type::MotionType::Kick),
            MotionCommand::Walk { .. } => Some(types::motion_type::MotionType::Walk),
            MotionCommand::Stand { .. } => Some(types::motion_type::MotionType::Stand),
            MotionCommand::StandUp { .. } => Some(types::motion_type::MotionType::StandUp),
            MotionCommand::Prepare => Some(types::motion_type::MotionType::Prepare),
            MotionCommand::Damping => Some(types::motion_type::MotionType::Damping),
            _ => None,
        };

        if motion_type != self.blackboard.last_motion_type {
            self.blackboard.last_motion_switch_time = self.blackboard.world_state.now;
            self.blackboard.last_motion_type = motion_type;
        }

        Ok(SimulatorBehaviorTickOutput {
            motion_command,
            trace,
            static_layout: self.static_layout.clone(),
            path_obstacles: self.blackboard.path_obstacles_output.clone(),
            time_since_last_switch: self.blackboard.time_since_last_switch,
            direction_difference: self.blackboard.direction_difference,
            walk_position: self.blackboard.walk_position,
            voronoi_map: self.blackboard.voronoi_map.clone(),
            voronoi_inputs: self.blackboard.voronoi_inputs.clone(),
        })
    }

    pub fn plan_communication(
        &mut self,
        world_state: WorldState,
        hsl_network_parameters: HslNetworkParameters,
        game_controller_address: Option<SocketAddr>,
    ) -> Vec<OutgoingMessage> {
        self.blackboard.world_state = world_state;
        self.blackboard.parameters.network = hsl_network_parameters;

        let mut outgoing_messages = Vec::new();
        if let Some(message) = self
            .blackboard
            .game_controller_return_message(game_controller_address.as_ref())
        {
            outgoing_messages.push(message);
        }
        if let Some(message) = self.blackboard.try_sending_state_message() {
            outgoing_messages.push(message);
        }
        outgoing_messages
    }
}

pub struct SimulatorBehaviorTickInput {
    pub world_state: WorldState,
    pub visual_kick_ball_position: Option<BallPosition<Ground>>,
    pub field_dimensions: FieldDimensions,
    pub parameters: BehaviorParameters,
}

pub struct SimulatorBehaviorTickOutput {
    pub motion_command: MotionCommand,
    pub trace: NodeTrace,
    pub static_layout: NodeTrace,
    pub path_obstacles: Vec<PathObstacle>,
    pub time_since_last_switch: Duration,
    pub direction_difference: f32,
    pub walk_position: Option<Point2<Ground>>,
    pub voronoi_map: Option<VoronoiGrid>,
    pub voronoi_inputs: Vec<Pose2<Field>>,
}

fn create_behavior_blackboard(parameters: BehaviorParameters) -> BehaviorBlackboard {
    BehaviorBlackboard {
        field_dimensions: FieldDimensions::default(),
        parameters,
        world_state: WorldState::default(),
        path_obstacles_output: Vec::new(),
        time_since_last_switch: Duration::ZERO,
        direction_difference: 0.0,
        voronoi_inputs: Vec::new(),
        ball: None,
        visual_kick_ball_position: None,
        last_ball: None,
        last_close_enough_to_kick: false,
        kick_target: None,
        last_kick_target: None,
        last_motion_command: MotionCommand::default(),
        last_motion_switch_time: ros_z::time::Time::zero(),
        last_motion_type: None,
        last_sent_game_controller_return_message_time: None,
        last_sent_hsl_message_time: None,
        last_closest_to_ball: false,
        closest_to_ball_entered_area_since: None,
        closest_to_ball_left_area_since: None,
        is_injected_motion_command: false,
        walk_position: None,
        body_motion: None,
        head_motion: None,
        voronoi_map: None,
    }
}

pub fn tick_behavior_trees(
    clock: Res<SimulatorClock>,
    field_dimensions: Res<SimulatorFieldDimensions>,
    world_states: Res<SimulatorWorldStates>,
    mut robot_frames: ResMut<SimulatorRobotFrames>,
    mut current_violations: ResMut<SimulatorCurrentInvariantViolations>,
    mut scenario_result: ResMut<SimulatorScenarioResult>,
    mut exit: MessageWriter<AppExit>,
    mut robots: Query<(
        &SimulatorRobot,
        &SimulatorRobotParameters,
        &mut SimulatorRobotBehavior,
    )>,
) {
    robot_frames.0.clear();

    for (robot, parameters, mut behavior) in &mut robots {
        let robot_id = robot.id();
        let Some(world_state) = world_states.0.get(&robot_id).cloned() else {
            continue;
        };

        let tick_output = match behavior.tick_behavior_tree(SimulatorBehaviorTickInput {
            visual_kick_ball_position: visual_kick_ball_position_from_world_state(&world_state),
            world_state: world_state.clone(),
            field_dimensions: field_dimensions.0,
            parameters: parameters.behavior.clone(),
        }) {
            Ok(tick_output) => tick_output,
            Err(error) => {
                scenario_result.failed = true;
                current_violations.0.push(InvariantViolation {
                    check_name: BEHAVIOR_TICK_ERROR_CHECK_NAME,
                    player_number: Some(robot.player_number),
                    message: behavior_tick_failure_message(robot_id, &clock, &error),
                    severity: InvariantSeverity::Error,
                });
                exit.write(AppExit::Success);
                return;
            }
        };
        robot_frames.0.insert(
            robot_id,
            RobotFrame::from_outputs(world_state, tick_output, Vec::new()),
        );
    }
}

fn visual_kick_ball_position_from_world_state(
    world_state: &WorldState,
) -> Option<BallPosition<Ground>> {
    world_state.ball.map(|ball| BallPosition {
        position: ball.ball_in_ground,
        velocity: Vector2::zeros(),
        last_seen: world_state.now,
    })
}

fn behavior_tick_failure_message(
    robot_id: crate::behavior_tree_simulator::SimulatorRobotId,
    clock: &SimulatorClock,
    error: &color_eyre::Report,
) -> String {
    let tick = clock
        .now
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| {
            let tick_duration = clock.tick_duration.as_nanos();
            (tick_duration != 0).then_some(elapsed.as_nanos() / tick_duration)
        });
    match tick {
        Some(tick) => format!(
            "behavior tick failed for robot {robot_id} at tick {tick} ({:?}): {error:#}",
            clock.now,
        ),
        None => format!(
            "behavior tick failed for robot {robot_id} at {:?}: {error:#}",
            clock.now,
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, time::Duration, time::SystemTime};

    use bevy::{app::App, ecs::message::Messages};
    use hsl_network_messages::{PlayerNumber, Team};
    use types::{behavior_tree::Status, world_state::WorldState};

    use super::*;
    use crate::behavior_tree_simulator::{
        DEFAULT_TICK_DURATION, SimulatorFieldDimensions, default_behavior_parameters,
        default_walking_parameters,
    };

    fn kick_blackboard(ball_position: Point2<Ground>) -> BehaviorBlackboard {
        let mut blackboard = create_behavior_blackboard(default_behavior_parameters().unwrap());
        blackboard.field_dimensions = FieldDimensions::SPL_2025;
        blackboard.world_state.robot.ground_to_field = Some(linear_algebra::Isometry2::identity());
        blackboard.visual_kick_ball_position = Some(BallPosition {
            position: ball_position,
            velocity: Vector2::zeros(),
            last_seen: ros_z::time::Time::zero(),
        });
        blackboard
    }

    #[test]
    fn strong_kick_selection_uses_ball_to_target_distance() {
        use behavior_node::kick::{apply_kick_target, kick, kick_strength_subtree};
        use linear_algebra::point;
        use types::motion_command::BodyMotion;

        for (ball_x, target_x, expected_strong) in [
            (1.0, 6.0, false), // Robot-to-target distance alone would enable strong.
            (-1.0, 5.0, true), // Ball-to-target distance reaches the threshold.
            (1.0, 7.0, true),
            (1.0, 6.9, false),
        ] {
            let mut blackboard = kick_blackboard(point![ball_x, 0.0]);
            blackboard.parameters.kicking.allow_strong_kicks = true;
            blackboard.parameters.kicking.target_speed = 1.2;
            assert_eq!(kick(&mut blackboard), Status::Success);
            assert_eq!(
                apply_kick_target(&mut blackboard, point![target_x, 0.0]),
                Status::Success
            );
            let (status, _) = kick_strength_subtree().tick_with_trace(&mut blackboard);
            assert_eq!(status, Status::Success);
            let Some(BodyMotion::Kick {
                strong,
                target_speed,
                kick_direction,
                ..
            }) = blackboard.body_motion
            else {
                panic!("expected kick command")
            };
            assert_eq!(strong, expected_strong);
            assert_eq!(target_speed, 1.2);
            assert_eq!(blackboard.kick_target, Some(point![target_x, 0.0]));
            assert_eq!(kick_direction.angle(), 0.0);

            blackboard.parameters.kicking.allow_strong_kicks = false;
            kick_strength_subtree().tick_with_trace(&mut blackboard);
            assert!(matches!(
                blackboard.body_motion,
                Some(BodyMotion::Kick {
                    strong: false,
                    target_speed: 1.2,
                    ..
                })
            ));
        }
    }

    #[test]
    fn kick_target_stays_in_behavior_and_is_reset_for_a_new_request() {
        use behavior_node::kick::{apply_kick_target, is_target_in_strong_kick_range, kick};
        use linear_algebra::point;

        let mut blackboard = kick_blackboard(point![0.3, 0.1]);
        assert_eq!(kick(&mut blackboard), Status::Success);
        assert_eq!(
            apply_kick_target(&mut blackboard, point![8.0, 0.0]),
            Status::Success
        );
        assert!(is_target_in_strong_kick_range(&mut blackboard));
        let command = assemble_motion_command(&blackboard, Status::Success).unwrap();
        let serialized = serde_json::to_value(command).unwrap();
        let kick_fields = serialized["Kick"].as_object().unwrap();
        assert!(kick_fields.contains_key("kick_direction"));
        assert!(!kick_fields.contains_key("target_position"));
        assert!(!kick_fields.contains_key("robot_theta_to_field"));
        assert_eq!(blackboard.kick_target, Some(point![8.0, 0.0]));

        assert_eq!(kick(&mut blackboard), Status::Success);
        assert_eq!(blackboard.kick_target, None);
        assert!(!is_target_in_strong_kick_range(&mut blackboard));
    }

    #[test]
    fn interception_cutoff_is_independent_of_approach_standoff() {
        use behavior_node::kick::{intercept, kick};
        use linear_algebra::{point, vector};
        use types::world_state::BallState;

        let mut blackboard = kick_blackboard(point![0.5, 1.0]);
        blackboard.world_state.ball = Some(BallState {
            ball_in_ground_velocity: vector![0.0, -1.0],
            ..Default::default()
        });
        blackboard
            .parameters
            .ball
            .interception
            .maximum_intercept_distance = 2.0;
        blackboard
            .parameters
            .kicking
            .minimum_interception_forward_distance = 0.4;
        for standoff in [0.3, 1.5] {
            blackboard.parameters.kicking.approach_ball_standoff = standoff;
            assert_eq!(kick(&mut blackboard), Status::Success);
            assert_eq!(intercept(&mut blackboard), Status::Success);
        }
        blackboard
            .parameters
            .kicking
            .minimum_interception_forward_distance = 0.6;
        assert_eq!(intercept(&mut blackboard), Status::Failure);
    }

    #[test]
    fn approach_alignment_uses_standoff_independently_of_interception_cutoff() {
        use behavior_node::{conditions::is_close_to_ball_aligned, node::LastBall};
        use linear_algebra::{point, vector};
        use types::field_dimensions::Side;

        let mut blackboard = kick_blackboard(point![1.0, 0.0]);
        blackboard.ball = Some(LastBall {
            position: point![1.0, 0.0],
            velocity: vector![0.0, 0.0],
            age: ros_z::time::Time::zero(),
            field_side: Side::Left,
        });
        blackboard
            .parameters
            .kicking
            .minimum_interception_forward_distance = 5.0;
        blackboard.parameters.kicking.approach_ball_standoff = 0.3;
        assert!(!is_close_to_ball_aligned(&mut blackboard));
        blackboard.parameters.kicking.approach_ball_standoff = 1.0;
        assert!(is_close_to_ball_aligned(&mut blackboard));
    }

    #[test]
    fn behavior_tick_failure_marks_scenario_failed_and_exits() {
        let mut app = App::new();
        app.add_message::<AppExit>()
            .insert_resource(SimulatorClock {
                now: SystemTime::UNIX_EPOCH + Duration::from_millis(30),
                tick_duration: DEFAULT_TICK_DURATION,
            })
            .insert_resource(SimulatorFieldDimensions(FieldDimensions::SPL_2025))
            .insert_resource(SimulatorWorldStates(BTreeMap::from([(
                crate::behavior_tree_simulator::SimulatorRobotId::new(
                    Team::Hulks,
                    PlayerNumber::Three,
                ),
                WorldState::default(),
            )])))
            .insert_resource(SimulatorRobotFrames::default())
            .insert_resource(SimulatorCurrentInvariantViolations::default())
            .insert_resource(SimulatorScenarioResult::default())
            .add_systems(Update, tick_behavior_trees);

        let mut behavior = SimulatorRobotBehavior::new(
            default_behavior_parameters().expect("failed to load behavior parameters"),
        );
        behavior.tree = BehaviorNodeTree::Action {
            name: "return_idle",
            action: Box::new(|_| Status::Idle),
        };
        app.world_mut().spawn((
            SimulatorRobot {
                team: Team::Hulks,
                player_number: PlayerNumber::Three,
            },
            SimulatorRobotParameters {
                behavior: default_behavior_parameters()
                    .expect("failed to load behavior parameters"),
                walking: default_walking_parameters().expect("failed to load walking parameters"),
            },
            behavior,
        ));

        let mut exit_cursor = app
            .world_mut()
            .resource_mut::<Messages<AppExit>>()
            .get_cursor();

        app.update();

        let scenario_result = app.world().resource::<SimulatorScenarioResult>();
        assert!(scenario_result.failed);
        assert!(scenario_result.failures.is_empty());

        let current_violations = app
            .world()
            .resource::<SimulatorCurrentInvariantViolations>();
        let [violation] = current_violations.0.as_slice() else {
            panic!("expected one current invariant violation");
        };
        assert_eq!(violation.check_name, BEHAVIOR_TICK_ERROR_CHECK_NAME);
        assert_eq!(violation.player_number, Some(PlayerNumber::Three));
        assert_eq!(violation.severity, InvariantSeverity::Error);
        assert!(violation.message.contains("robot H3"));
        assert!(violation.message.contains("tick 3"));
        assert!(
            violation
                .message
                .contains("Behavior tree returned Idle status")
        );

        let exits = exit_cursor
            .read(app.world().resource::<Messages<AppExit>>())
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(exits, vec![AppExit::Success]);
    }
}
