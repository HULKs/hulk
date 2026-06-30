use std::{env, time::Duration, time::SystemTime};

use bevy::{
    app::{App, AppExit, Plugin, Update},
    ecs::message::Messages,
    prelude::*,
};
use color_eyre::{Result, eyre::bail};
use coordinate_systems::{Ground, World};
use linear_algebra::{Isometry2, Point2};
use serde::Serialize;
use types::{
    field_dimensions::FieldDimensions,
    obstacles::{Obstacle, ObstacleKind},
    parameters::HslNetworkParameters,
    rule_obstacles::RuleObstacle,
};

use crate::timeline_viewer::{TimelineViewerData, show_timeline_viewer};

pub use crate::auto_referee::{
    AutoRefereeConfig, AutoRefereeContext, AutoRefereeRule, AutoRefereeState,
    GameStateTransitionRule, HalftimeTimeoutRule, ScoredGoalRule, SimulatorAutoReferee,
    SimulatorRefereeCommand, SimulatorRestartReason,
};
pub use crate::ball::{SimulatedBall, SimulatorBall};
pub use crate::behavior_runtime::{
    SimulatorBehaviorTickInput, SimulatorBehaviorTickOutput, SimulatorRobotBehavior,
};
pub use crate::communication::{
    SimulatorHslNetworkParameters, SimulatorIncomingMessage, SimulatorIncomingMessages,
    SimulatorMessage, SimulatorOutgoingMessages, SimulatorReceivedHslMessage,
    SimulatorReceivedHslMessages,
};
pub use crate::config::{
    DEFAULT_TICK_DURATION, SimulationConfig, default_behavior_parameters,
    default_walking_parameters,
};
pub use crate::game_controller::SimulatorGameState;
pub use crate::invariant_checks::{
    InvariantCheck, InvariantSeverity, InvariantViolation, RobotSnapshot, SimulationSnapshot,
    SimulatorCurrentInvariantViolations, SimulatorInvariantChecks, default_invariant_checks,
};
pub use crate::robot::{
    SimulatorFallDownState, SimulatorGroundToWorld, SimulatorHeadYaw, SimulatorLastKickTime,
    SimulatorPrimaryState, SimulatorRobot, SimulatorRobotBundle, SimulatorRobotId,
    SimulatorRobotParameters, SimulatorSuggestedSearchPosition,
};
pub use crate::timeline::{
    RobotFrame, SimulatorFailure, SimulatorRobotFrames, SimulatorScenarioResult, SimulatorTimeline,
    SimulatorTimelineMarker, SimulatorTimelineMarkers, TimelineFrame,
};
pub use crate::world_states::SimulatorWorldStates;

pub use crate::auto_referee::run_auto_referee;
pub use crate::ball::{move_ball, update_ball_last_touch_from_robot_contacts};
pub use crate::behavior_runtime::tick_behavior_trees;
pub use crate::communication::{
    apply_incoming_hsl_messages, plan_communication, route_outgoing_communication,
};
pub use crate::coordinates::point_world_to_field;
pub use crate::game_controller::sync_primary_states_from_game_state;
pub use crate::invariant_checks::run_invariant_checks;
pub use crate::kinematics::move_robots;
pub use crate::timeline::record_timeline_frame;
pub use crate::world_states::build_world_states;
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BehaviorTreeSimulatorSet {
    AdvanceTime,
    BeforeBallPhysics,
    BallPhysics,
    AfterBallPhysics,
    BeforeAutoReferee,
    RunAutoReferee,
    AfterAutoReferee,
    BuildTeamContext,
    BeforeWorldState,
    BuildWorldStates,
    AfterWorldState,
    BeforeBehavior,
    TickBehaviorTrees,
    AfterBehavior,
    BeforeCommunication,
    PlanCommunication,
    AfterCommunication,
    BeforeKinematics,
    ApplyKinematics,
    AfterKinematics,
    BeforeInvariantChecks,
    RunInvariantChecks,
    AfterInvariantChecks,
    RecordTimeline,
    Scenario,
}

pub struct BehaviorTreeSimulatorPlugin {
    pub config: SimulationConfig,
    pub auto_referee_config: AutoRefereeConfig,
    pub field_dimensions: FieldDimensions,
    pub hsl_network_parameters: HslNetworkParameters,
    pub tick_duration: Duration,
    pub enable_default_ball_physics: bool,
    pub enable_default_kinematics: bool,
    pub enable_default_communication_routing: bool,
    pub enable_default_invariant_checks: bool,
}

impl Default for BehaviorTreeSimulatorPlugin {
    fn default() -> Self {
        Self {
            config: SimulationConfig::default(),
            auto_referee_config: AutoRefereeConfig::default(),
            field_dimensions: FieldDimensions::SPL_2025,
            hsl_network_parameters: HslNetworkParameters::default(),
            tick_duration: DEFAULT_TICK_DURATION,
            enable_default_ball_physics: true,
            enable_default_kinematics: true,
            enable_default_communication_routing: true,
            enable_default_invariant_checks: true,
        }
    }
}

impl Plugin for BehaviorTreeSimulatorPlugin {
    fn build(&self, app: &mut App) {
        let mut game_state = SimulatorGameState::default();
        game_state
            .game_controller_state
            .hulks_team
            .remaining_amount_of_messages = self.config.remaining_amount_of_messages.unwrap_or(0);
        game_state.sync_filtered_game_controller_state();

        app.add_message::<AppExit>()
            .add_message::<SimulatorRefereeCommand>()
            .insert_resource(SimulatorClock {
                now: SystemTime::UNIX_EPOCH,
                tick_duration: self.tick_duration,
            })
            .insert_resource(SimulatorFieldDimensions(self.field_dimensions))
            .insert_resource(SimulatorBall::default())
            .insert_resource(game_state)
            .insert_resource(SimulatorAutoReferee::default())
            .insert_resource(SimulatorRuleObstacles::default())
            .insert_resource(SimulatorScenarioObstacles::default())
            .insert_resource(SimulatorHslNetworkParameters(
                self.hsl_network_parameters.clone(),
            ))
            .insert_resource(self.config.clone())
            .insert_resource(self.auto_referee_config.clone())
            .insert_resource(SimulatorTimeline::default())
            .insert_resource(SimulatorTimelineMarkers::default())
            .insert_resource(SimulatorScenarioResult::default())
            .insert_resource(SimulatorIncomingMessages::default())
            .insert_resource(SimulatorOutgoingMessages::default())
            .insert_resource(SimulatorReceivedHslMessages::default())
            .insert_resource(SimulatorWorldStates::default())
            .insert_resource(SimulatorRobotFrames::default())
            .insert_resource(SimulatorCurrentInvariantViolations::default());

        if self.enable_default_invariant_checks {
            app.insert_resource(SimulatorInvariantChecks(default_invariant_checks()));
        } else {
            app.insert_resource(SimulatorInvariantChecks::default());
        }

        app.configure_sets(
            Update,
            (
                BehaviorTreeSimulatorSet::AdvanceTime,
                BehaviorTreeSimulatorSet::BeforeBallPhysics,
                BehaviorTreeSimulatorSet::BallPhysics,
                BehaviorTreeSimulatorSet::AfterBallPhysics,
                BehaviorTreeSimulatorSet::BeforeAutoReferee,
                BehaviorTreeSimulatorSet::RunAutoReferee,
                BehaviorTreeSimulatorSet::AfterAutoReferee,
                BehaviorTreeSimulatorSet::BuildTeamContext,
                BehaviorTreeSimulatorSet::BeforeWorldState,
                BehaviorTreeSimulatorSet::BuildWorldStates,
                BehaviorTreeSimulatorSet::AfterWorldState,
                BehaviorTreeSimulatorSet::BeforeBehavior,
                BehaviorTreeSimulatorSet::TickBehaviorTrees,
                BehaviorTreeSimulatorSet::AfterBehavior,
                BehaviorTreeSimulatorSet::BeforeCommunication,
                BehaviorTreeSimulatorSet::PlanCommunication,
                BehaviorTreeSimulatorSet::AfterCommunication,
            )
                .chain(),
        )
        .configure_sets(
            Update,
            (
                BehaviorTreeSimulatorSet::BeforeInvariantChecks,
                BehaviorTreeSimulatorSet::RunInvariantChecks,
                BehaviorTreeSimulatorSet::AfterInvariantChecks,
                BehaviorTreeSimulatorSet::BeforeKinematics,
                BehaviorTreeSimulatorSet::ApplyKinematics,
                BehaviorTreeSimulatorSet::AfterKinematics,
                BehaviorTreeSimulatorSet::RecordTimeline,
                BehaviorTreeSimulatorSet::Scenario,
            )
                .chain(),
        )
        .configure_sets(
            Update,
            BehaviorTreeSimulatorSet::BeforeKinematics
                .after(BehaviorTreeSimulatorSet::AfterCommunication),
        )
        .add_systems(
            Update,
            advance_time.in_set(BehaviorTreeSimulatorSet::AdvanceTime),
        )
        .add_systems(
            Update,
            build_world_states.in_set(BehaviorTreeSimulatorSet::BuildWorldStates),
        )
        .add_systems(
            Update,
            update_ball_last_touch_from_robot_contacts
                .in_set(BehaviorTreeSimulatorSet::BeforeAutoReferee),
        )
        .add_systems(
            Update,
            run_auto_referee.in_set(BehaviorTreeSimulatorSet::RunAutoReferee),
        )
        .add_systems(
            Update,
            sync_primary_states_from_game_state.in_set(BehaviorTreeSimulatorSet::BeforeWorldState),
        )
        .add_systems(
            Update,
            apply_incoming_hsl_messages.in_set(BehaviorTreeSimulatorSet::BuildTeamContext),
        )
        .add_systems(
            Update,
            tick_behavior_trees.in_set(BehaviorTreeSimulatorSet::TickBehaviorTrees),
        )
        .add_systems(
            Update,
            plan_communication.in_set(BehaviorTreeSimulatorSet::PlanCommunication),
        )
        .add_systems(
            Update,
            run_invariant_checks.in_set(BehaviorTreeSimulatorSet::RunInvariantChecks),
        )
        .add_systems(
            Update,
            record_timeline_frame.in_set(BehaviorTreeSimulatorSet::RecordTimeline),
        );

        if self.enable_default_ball_physics {
            app.add_systems(
                Update,
                move_ball.in_set(BehaviorTreeSimulatorSet::BallPhysics),
            );
        }

        if self.enable_default_communication_routing {
            app.add_systems(
                Update,
                route_outgoing_communication.in_set(BehaviorTreeSimulatorSet::AfterCommunication),
            );
        }

        if self.enable_default_kinematics {
            app.add_systems(
                Update,
                move_robots.in_set(BehaviorTreeSimulatorSet::ApplyKinematics),
            );
        }
    }
}

pub trait AppExt {
    fn run_to_completion(&mut self) -> Result<()>;
    fn run_to_completion_with_viewer(&mut self) -> Result<()>;
}

impl AppExt for App {
    fn run_to_completion(&mut self) -> Result<()> {
        let exit = run_until_exit(self);
        check_scenario_result(self, exit)
    }

    fn run_to_completion_with_viewer(&mut self) -> Result<()> {
        let exit = run_until_exit(self);

        if env::var_os("BEVYHAVIOR_SIMULATOR_NO_VIEWER").is_none() {
            let viewer_data = TimelineViewerData {
                field_dimensions: self.world().resource::<SimulatorFieldDimensions>().0,
                config: self.world().resource::<SimulationConfig>().clone(),
                frames: std::mem::take(
                    &mut self.world_mut().resource_mut::<SimulatorTimeline>().frames,
                ),
                markers: self
                    .world()
                    .resource::<SimulatorTimelineMarkers>()
                    .markers
                    .clone(),
                failures: self
                    .world()
                    .resource::<SimulatorScenarioResult>()
                    .failures
                    .clone(),
            };

            show_timeline_viewer(viewer_data)?;
        }

        check_scenario_result(self, exit)
    }
}

fn run_until_exit(app: &mut App) -> AppExit {
    let mut event_cursor = app
        .world_mut()
        .resource_mut::<Messages<AppExit>>()
        .get_cursor();

    loop {
        app.update();

        let events = app.world().resource::<Messages<AppExit>>();
        if let Some(exit_message) = event_cursor.read(events).last() {
            break exit_message.clone();
        }
    }
}

fn check_scenario_result(app: &App, exit: AppExit) -> Result<()> {
    if let AppExit::Error(code) = exit {
        bail!("scenario exited with error code {code}");
    }

    let scenario_result = app.world().resource::<SimulatorScenarioResult>();
    if scenario_result.failed {
        bail!("{} simulator failure(s)", scenario_result.failures.len());
    }

    Ok(())
}

#[derive(Resource, Clone, Copy, Debug)]
pub struct SimulatorClock {
    pub now: SystemTime,
    pub tick_duration: Duration,
}

#[derive(Resource, Clone, Copy, Debug)]
pub struct SimulatorFieldDimensions(pub FieldDimensions);

#[derive(Resource, Clone, Debug, Default)]
pub struct SimulatorRuleObstacles {
    pub obstacles: Vec<RuleObstacle>,
}

#[derive(Resource, Clone, Debug, Default)]
pub struct SimulatorScenarioObstacles {
    pub obstacles: Vec<SimulatorObstacle>,
}

impl SimulatorScenarioObstacles {
    pub fn add(&mut self, obstacle: SimulatorObstacle) {
        self.obstacles.push(obstacle);
    }

    pub fn clear(&mut self) {
        self.obstacles.clear();
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct SimulatorObstacle {
    pub kind: ObstacleKind,
    pub position: Point2<World>,
    pub radius_at_foot_height: f32,
    pub radius_at_hip_height: f32,
}

impl SimulatorObstacle {
    pub fn new(
        kind: ObstacleKind,
        position: Point2<World>,
        radius_at_foot_height: f32,
        radius_at_hip_height: f32,
    ) -> Self {
        Self {
            kind,
            position,
            radius_at_foot_height,
            radius_at_hip_height,
        }
    }

    pub fn robot(
        position: Point2<World>,
        radius_at_foot_height: f32,
        radius_at_hip_height: f32,
    ) -> Self {
        Self::new(
            ObstacleKind::Robot,
            position,
            radius_at_foot_height,
            radius_at_hip_height,
        )
    }

    pub fn person(
        position: Point2<World>,
        radius_at_foot_height: f32,
        radius_at_hip_height: f32,
    ) -> Self {
        Self::new(
            ObstacleKind::Person,
            position,
            radius_at_foot_height,
            radius_at_hip_height,
        )
    }

    pub fn unknown(position: Point2<World>, radius_at_foot_height: f32) -> Self {
        Self::new(
            ObstacleKind::Unknown,
            position,
            radius_at_foot_height,
            radius_at_foot_height,
        )
    }

    pub fn to_world_state_obstacle(self, ground_to_world: Isometry2<Ground, World>) -> Obstacle {
        Obstacle {
            kind: self.kind,
            position: ground_to_world.inverse() * self.position,
            radius_at_foot_height: self.radius_at_foot_height,
            radius_at_hip_height: self.radius_at_hip_height,
        }
    }
}

fn advance_time(mut clock: ResMut<SimulatorClock>) {
    let tick_duration = clock.tick_duration;
    clock.now += tick_duration;
}

#[cfg(test)]
mod tests {
    use super::*;

    use hsl_network_messages::{PlayerNumber, Team};
    use linear_algebra::{Isometry2, point, vector};
    use types::{
        field_dimensions::Side, motion_command::MotionCommand, primary_state::PrimaryState,
    };

    #[test]
    fn plugin_initializes_live_message_budget_from_simulation_config() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            BehaviorTreeSimulatorPlugin {
                config: SimulationConfig {
                    remaining_amount_of_messages: Some(7),
                    ..Default::default()
                },
                ..Default::default()
            },
        ));

        let game_state = app.world().resource::<SimulatorGameState>();
        assert_eq!(
            game_state
                .game_controller_state
                .hulks_team
                .remaining_amount_of_messages,
            7
        );
        assert_eq!(
            game_state
                .filtered_game_controller_state
                .as_ref()
                .expect("filtered game state should exist")
                .remaining_number_of_messages,
            7
        );
    }

    #[test]
    fn scored_goal_updates_robot_primary_state_to_ready() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, BehaviorTreeSimulatorPlugin::default()));

        let field_dimensions = app.world().resource::<SimulatorFieldDimensions>().0;
        let parameters = default_behavior_parameters().expect("failed to load behavior parameters");
        app.world_mut().spawn(
            SimulatorRobotBundle::new(
                Team::Hulks,
                PlayerNumber::Three,
                Isometry2::identity(),
                parameters,
            )
            .expect("failed to create robot bundle")
            .with_primary_state(PrimaryState::Playing),
        );
        app.world_mut().resource_mut::<SimulatorBall>().state = Some(SimulatedBall {
            position: point![field_dimensions.length / 2.0 + 0.1, 0.0],
            velocity: vector![0.0, 0.0],
            field_side: Side::Left,
        });

        app.update();

        let mut query = app.world_mut().query::<&SimulatorPrimaryState>();
        let primary_states = query
            .iter(app.world())
            .map(|primary_state| primary_state.primary_state)
            .collect::<Vec<_>>();
        assert_eq!(primary_states, vec![PrimaryState::Ready]);

        let robot_frames = app.world().resource::<SimulatorRobotFrames>();
        let robot_frame = robot_frames
            .0
            .get(&SimulatorRobotId::new(Team::Hulks, PlayerNumber::Three))
            .expect("robot should have ticked behavior");
        assert!(matches!(
            robot_frame.motion_command,
            MotionCommand::Walk { .. }
        ));
    }
}
