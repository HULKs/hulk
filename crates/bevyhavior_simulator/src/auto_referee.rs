use std::{collections::BTreeMap, time::Duration, time::SystemTime};

use bevy::prelude::*;
use coordinate_systems::{Ground, World};
use hsl_network_messages::{GamePhase, GameState, Team};
use linear_algebra::{Isometry2, Point2, Vector2, distance};
use types::{
    field_dimensions::{FieldDimensions, GlobalFieldSide, Side},
    game_controller_state::GameControllerState,
};

use crate::behavior_tree_simulator::{
    SimulatedBall, SimulatorBall, SimulatorFieldDimensions, SimulatorGameState,
    SimulatorGroundToWorld, SimulatorRobot, SimulatorRobotId, point_world_to_field,
};

const READY_STATIONARY_TRANSLATION_EPSILON: f32 = 0.01;
const READY_STATIONARY_ROTATION_EPSILON: f32 = 0.01;

#[derive(Resource, Clone, Debug)]
pub struct AutoRefereeConfig {
    pub ready_duration: Duration,
    pub ready_stationary_short_circuit_duration: Option<Duration>,
    pub whistle_to_playing_delay: Duration,
    pub halftime_duration: Duration,
    pub auto_whistle_in_set: bool,
    pub finish_on_halftime_timeout: bool,
}

impl Default for AutoRefereeConfig {
    fn default() -> Self {
        Self {
            ready_duration: Duration::from_secs(45),
            ready_stationary_short_circuit_duration: Some(Duration::from_secs(1)),
            whistle_to_playing_delay: Duration::from_secs(3),
            halftime_duration: Duration::from_secs(10 * 60),
            auto_whistle_in_set: true,
            finish_on_halftime_timeout: true,
        }
    }
}

#[derive(Resource)]
pub struct SimulatorAutoReferee {
    pub rules: Vec<Box<dyn AutoRefereeRule>>,
    pub state: AutoRefereeState,
}

#[derive(Clone, Debug)]
pub struct AutoRefereeState {
    pub last_game_state_change: SystemTime,
    pub halftime_started_at: Option<SystemTime>,
    pub playing_after_whistle_at: Option<SystemTime>,
    pub restart_reason: Option<SimulatorRestartReason>,
    pub ready_stationary_since: Option<SystemTime>,
    pub ready_robot_poses: BTreeMap<SimulatorRobotId, Isometry2<Ground, World>>,
}

impl Default for AutoRefereeState {
    fn default() -> Self {
        Self {
            last_game_state_change: SystemTime::UNIX_EPOCH,
            halftime_started_at: None,
            playing_after_whistle_at: None,
            restart_reason: None,
            ready_stationary_since: None,
            ready_robot_poses: BTreeMap::new(),
        }
    }
}

impl AutoRefereeState {
    fn reset_ready_stationary_tracking(&mut self) {
        self.ready_stationary_since = None;
        self.ready_robot_poses.clear();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimulatorRestartReason {
    KickOffAfterGoal { scoring_team: Team },
    DroppedBall,
}

#[derive(Clone, Copy, Debug, Message)]
pub enum SimulatorRefereeCommand {
    SetGameState(GameState),
    Whistle,
    BriefStop,
    Resume,
    DroppedBall,
    SetTimeout(bool),
}

pub trait AutoRefereeRule: Send + Sync {
    fn apply(&mut self, context: &mut AutoRefereeContext<'_>);
}

pub struct AutoRefereeContext<'a> {
    pub now: SystemTime,
    pub config: &'a AutoRefereeConfig,
    pub field_dimensions: FieldDimensions,
    pub game_state: &'a mut SimulatorGameState,
    pub auto_referee: &'a mut AutoRefereeState,
    pub ball: &'a mut SimulatorBall,
    pub robot_poses: BTreeMap<SimulatorRobotId, Isometry2<Ground, World>>,
}

impl AutoRefereeContext<'_> {
    fn set_game_state(&mut self, game_state: GameState) {
        self.game_state.set_game_state(game_state, self.now);
        self.auto_referee.last_game_state_change = self.now;
        self.auto_referee.reset_ready_stationary_tracking();

        if game_state != GameState::Set {
            self.auto_referee.playing_after_whistle_at = None;
        }
    }

    fn set_kicking_team(&mut self, kicking_team: Option<Team>) {
        self.game_state.set_kicking_team(kicking_team);
    }
}

pub struct ScoredGoalRule;

impl AutoRefereeRule for ScoredGoalRule {
    fn apply(&mut self, context: &mut AutoRefereeContext<'_>) {
        if context.game_state.game_controller_state.game_state != GameState::Playing {
            return;
        }

        let Some(scoring_team) = context.ball.state.and_then(|ball| {
            ball_in_goal(
                ball,
                context.field_dimensions,
                context.game_state.game_controller_state.global_field_side,
            )
        }) else {
            return;
        };

        match scoring_team {
            Team::Hulks => {
                context.game_state.game_controller_state.hulks_team.score = context
                    .game_state
                    .game_controller_state
                    .hulks_team
                    .score
                    .saturating_add(1);
            }
            Team::Opponent => {
                context.game_state.game_controller_state.opponent_team.score = context
                    .game_state
                    .game_controller_state
                    .opponent_team
                    .score
                    .saturating_add(1);
            }
        }
        context.ball.state = None;

        if goal_difference(&context.game_state.game_controller_state) >= 10 {
            context.set_game_state(GameState::Finished);
            return;
        }

        context.set_kicking_team(Some(opponent_of(scoring_team)));
        context.auto_referee.restart_reason =
            Some(SimulatorRestartReason::KickOffAfterGoal { scoring_team });
        context.set_game_state(GameState::Ready);
    }
}

impl Default for ScoredGoalRule {
    fn default() -> Self {
        Self
    }
}

pub struct GameStateTransitionRule;

impl AutoRefereeRule for GameStateTransitionRule {
    fn apply(&mut self, context: &mut AutoRefereeContext<'_>) {
        match context.game_state.game_controller_state.game_state {
            GameState::Initial => {}
            GameState::Ready => {
                let ready_duration_elapsed = has_elapsed(
                    context.now,
                    context.auto_referee.last_game_state_change,
                    context.config.ready_duration,
                );
                let robots_are_stationary = ready_stationary_short_circuit_elapsed(context);
                if ready_duration_elapsed || robots_are_stationary {
                    if context.auto_referee.restart_reason.is_some() {
                        place_ball_at_center(context.ball);
                    }
                    context.set_game_state(GameState::Set);
                }
            }
            GameState::Set => match context.auto_referee.playing_after_whistle_at {
                Some(playing_after_whistle_at) if context.now >= playing_after_whistle_at => {
                    context.set_game_state(GameState::Playing);
                    context.auto_referee.playing_after_whistle_at = None;
                    context.auto_referee.restart_reason = None;
                    if context.auto_referee.halftime_started_at.is_none() {
                        context.auto_referee.halftime_started_at = Some(context.now);
                    }
                }
                Some(_) => {}
                None if context.config.auto_whistle_in_set => {
                    context.auto_referee.playing_after_whistle_at =
                        Some(context.now + context.config.whistle_to_playing_delay);
                }
                None => {}
            },
            GameState::Playing => {
                context.auto_referee.reset_ready_stationary_tracking();
                if context.auto_referee.halftime_started_at.is_none() {
                    context.auto_referee.halftime_started_at = Some(context.now);
                }
            }
            GameState::Finished => context.auto_referee.reset_ready_stationary_tracking(),
        }
    }
}

fn ready_stationary_short_circuit_elapsed(context: &mut AutoRefereeContext<'_>) -> bool {
    let Some(duration) = context.config.ready_stationary_short_circuit_duration else {
        context.auto_referee.reset_ready_stationary_tracking();
        return false;
    };
    if context.robot_poses.is_empty() {
        context.auto_referee.reset_ready_stationary_tracking();
        return false;
    }

    if robot_poses_are_stationary(
        &context.auto_referee.ready_robot_poses,
        &context.robot_poses,
    ) {
        let stationary_since = context
            .auto_referee
            .ready_stationary_since
            .unwrap_or(context.now);
        context.auto_referee.ready_stationary_since = Some(stationary_since);
        has_elapsed(context.now, stationary_since, duration)
    } else {
        context.auto_referee.ready_stationary_since = Some(context.now);
        context.auto_referee.ready_robot_poses = context.robot_poses.clone();
        false
    }
}

fn robot_poses_are_stationary(
    previous_poses: &BTreeMap<SimulatorRobotId, Isometry2<Ground, World>>,
    current_poses: &BTreeMap<SimulatorRobotId, Isometry2<Ground, World>>,
) -> bool {
    if previous_poses.len() != current_poses.len() {
        return false;
    }

    current_poses.iter().all(|(robot_id, current_pose)| {
        previous_poses
            .get(robot_id)
            .is_some_and(|previous_pose| robot_pose_is_stationary(*previous_pose, *current_pose))
    })
}

fn robot_pose_is_stationary(
    previous_pose: Isometry2<Ground, World>,
    current_pose: Isometry2<Ground, World>,
) -> bool {
    distance(previous_pose.translation(), current_pose.translation())
        <= READY_STATIONARY_TRANSLATION_EPSILON
        && (previous_pose.orientation().angle() - current_pose.orientation().angle()).abs()
            <= READY_STATIONARY_ROTATION_EPSILON
}

impl Default for GameStateTransitionRule {
    fn default() -> Self {
        Self
    }
}

pub struct HalftimeTimeoutRule;

impl AutoRefereeRule for HalftimeTimeoutRule {
    fn apply(&mut self, context: &mut AutoRefereeContext<'_>) {
        if !context.config.finish_on_halftime_timeout
            || context.game_state.game_controller_state.game_state != GameState::Playing
        {
            return;
        }

        let Some(halftime_started_at) = context.auto_referee.halftime_started_at else {
            return;
        };

        if has_elapsed(
            context.now,
            halftime_started_at,
            context.config.halftime_duration,
        ) {
            context.set_game_state(GameState::Finished);
        }
    }
}

impl Default for HalftimeTimeoutRule {
    fn default() -> Self {
        Self
    }
}

impl SimulatorAutoReferee {
    pub fn with_default_rules() -> Self {
        Self {
            rules: vec![
                Box::new(ScoredGoalRule),
                Box::new(GameStateTransitionRule),
                Box::new(HalftimeTimeoutRule),
            ],
            state: AutoRefereeState::default(),
        }
    }
}

impl Default for SimulatorAutoReferee {
    fn default() -> Self {
        Self::with_default_rules()
    }
}

pub fn run_auto_referee(
    clock: Res<crate::behavior_tree_simulator::SimulatorClock>,
    config: Res<AutoRefereeConfig>,
    field_dimensions: Res<SimulatorFieldDimensions>,
    mut auto_referee: ResMut<SimulatorAutoReferee>,
    mut game_state: ResMut<SimulatorGameState>,
    mut ball: ResMut<SimulatorBall>,
    mut referee_commands: MessageReader<SimulatorRefereeCommand>,
    robots: Query<(&SimulatorRobot, &SimulatorGroundToWorld)>,
) {
    let mut rules = std::mem::take(&mut auto_referee.rules);
    let robot_poses = robots
        .iter()
        .map(|(robot, ground_to_world)| (robot.id(), ground_to_world.ground_to_world))
        .collect();
    let mut context = AutoRefereeContext {
        now: clock.now,
        config: &config,
        field_dimensions: field_dimensions.0,
        game_state: &mut game_state,
        auto_referee: &mut auto_referee.state,
        ball: &mut ball,
        robot_poses,
    };

    for command in referee_commands.read() {
        apply_referee_command(*command, &mut context);
    }

    for rule in &mut rules {
        rule.apply(&mut context);
    }

    auto_referee.rules = rules;
}

fn apply_referee_command(command: SimulatorRefereeCommand, context: &mut AutoRefereeContext<'_>) {
    match command {
        SimulatorRefereeCommand::SetGameState(game_state) => {
            context.set_game_state(game_state);
            if game_state == GameState::Playing
                && context.auto_referee.halftime_started_at.is_none()
            {
                context.auto_referee.halftime_started_at = Some(context.now);
            }
        }
        SimulatorRefereeCommand::Whistle => {
            if context.game_state.game_controller_state.game_state == GameState::Set {
                context.auto_referee.playing_after_whistle_at =
                    Some(context.now + context.config.whistle_to_playing_delay);
            }
        }
        SimulatorRefereeCommand::BriefStop => context.game_state.set_stopped(true),
        SimulatorRefereeCommand::Resume => context.game_state.set_stopped(false),
        SimulatorRefereeCommand::DroppedBall => {
            context.set_kicking_team(None);
            context.auto_referee.restart_reason = Some(SimulatorRestartReason::DroppedBall);
            context.set_game_state(GameState::Ready);
        }
        SimulatorRefereeCommand::SetTimeout(active) => {
            context.game_state.set_game_phase(if active {
                GamePhase::Timeout
            } else {
                GamePhase::Normal
            });
        }
    }
}

fn opponent_of(team: Team) -> Team {
    match team {
        Team::Hulks => Team::Opponent,
        Team::Opponent => Team::Hulks,
    }
}

fn goal_difference(game_controller_state: &GameControllerState) -> u8 {
    game_controller_state
        .hulks_team
        .score
        .abs_diff(game_controller_state.opponent_team.score)
}

fn has_elapsed(now: SystemTime, since: SystemTime, duration: Duration) -> bool {
    matches!(now.duration_since(since), Ok(elapsed) if elapsed >= duration)
}

fn place_ball_at_center(ball: &mut SimulatorBall) {
    ball.state = Some(SimulatedBall {
        position: Point2::origin(),
        velocity: Vector2::zeros(),
        field_side: Side::Left,
    });
}

fn ball_in_goal(
    ball: SimulatedBall,
    field_dimensions: FieldDimensions,
    global_field_side: GlobalFieldSide,
) -> Option<Team> {
    if !field_dimensions.is_inside_any_goal(ball.position) {
        return None;
    }

    let ball_in_field = point_world_to_field(ball.position, global_field_side);
    if ball_in_field.x() > 0.0 {
        Some(Team::Hulks)
    } else {
        Some(Team::Opponent)
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, time::Duration, time::SystemTime};

    use hsl_network_messages::{GamePhase, GameState, PlayerNumber, Team};
    use linear_algebra::{Isometry2, Point2, point, vector};
    use types::{
        field_dimensions::{FieldDimensions, GlobalFieldSide, Side},
        filtered_game_state::FilteredGameState,
    };

    use super::*;
    use crate::behavior_tree_simulator::{SimulatedBall, SimulatorBall, SimulatorGameState};

    fn robot_id(player_number: PlayerNumber) -> SimulatorRobotId {
        SimulatorRobotId::new(Team::Hulks, player_number)
    }

    fn auto_referee_context<'a>(
        now: SystemTime,
        config: &'a AutoRefereeConfig,
        field_dimensions: FieldDimensions,
        game_state: &'a mut SimulatorGameState,
        auto_referee: &'a mut AutoRefereeState,
        ball: &'a mut SimulatorBall,
    ) -> AutoRefereeContext<'a> {
        auto_referee_context_with_robot_poses(
            now,
            config,
            field_dimensions,
            game_state,
            auto_referee,
            ball,
            BTreeMap::new(),
        )
    }

    fn auto_referee_context_with_robot_poses<'a>(
        now: SystemTime,
        config: &'a AutoRefereeConfig,
        field_dimensions: FieldDimensions,
        game_state: &'a mut SimulatorGameState,
        auto_referee: &'a mut AutoRefereeState,
        ball: &'a mut SimulatorBall,
        robot_poses: BTreeMap<SimulatorRobotId, Isometry2<Ground, World>>,
    ) -> AutoRefereeContext<'a> {
        AutoRefereeContext {
            now,
            config,
            field_dimensions,
            game_state,
            auto_referee,
            ball,
            robot_poses,
        }
    }

    fn transition_test_config() -> AutoRefereeConfig {
        AutoRefereeConfig {
            ready_duration: Duration::ZERO,
            ready_stationary_short_circuit_duration: Some(Duration::from_secs(1)),
            whistle_to_playing_delay: Duration::ZERO,
            halftime_duration: Duration::from_secs(600),
            auto_whistle_in_set: true,
            finish_on_halftime_timeout: true,
        }
    }

    #[test]
    fn ball_in_goal_uses_global_field_side() {
        let field_dimensions = FieldDimensions::SPL_2025;
        let ball_in_home_right_goal = SimulatedBall {
            position: point![field_dimensions.length / 2.0 + 0.1, 0.0],
            velocity: vector![0.0, 0.0],
            field_side: Side::Left,
        };

        assert_eq!(
            ball_in_goal(
                ball_in_home_right_goal,
                field_dimensions,
                GlobalFieldSide::Home
            ),
            Some(Team::Hulks)
        );
        assert_eq!(
            ball_in_goal(
                ball_in_home_right_goal,
                field_dimensions,
                GlobalFieldSide::Away
            ),
            Some(Team::Opponent)
        );
    }

    #[test]
    fn scored_goal_in_opponent_goal_increases_hulks_score_and_removes_ball() {
        let field_dimensions = FieldDimensions::SPL_2025;
        let config = AutoRefereeConfig::default();
        let mut game_state = SimulatorGameState::default();
        let mut auto_referee = AutoRefereeState::default();
        let mut ball = SimulatorBall {
            state: Some(SimulatedBall {
                position: point![field_dimensions.length / 2.0 + 0.1, 0.0],
                velocity: vector![0.0, 0.0],
                field_side: Side::Left,
            }),
        };
        let mut rule = ScoredGoalRule;

        rule.apply(&mut auto_referee_context(
            SystemTime::UNIX_EPOCH + Duration::from_secs(1),
            &config,
            field_dimensions,
            &mut game_state,
            &mut auto_referee,
            &mut ball,
        ));

        assert_eq!(game_state.game_controller_state.hulks_team.score, 1);
        assert_eq!(game_state.game_controller_state.opponent_team.score, 0);
        assert_eq!(
            game_state.game_controller_state.game_state,
            GameState::Ready
        );
        assert_eq!(
            game_state.game_controller_state.kicking_team,
            Some(Team::Opponent)
        );
        assert_eq!(
            auto_referee.restart_reason,
            Some(SimulatorRestartReason::KickOffAfterGoal {
                scoring_team: Team::Hulks,
            })
        );
        assert!(ball.state.is_none());
    }

    #[test]
    fn scored_goal_in_hulks_goal_increases_opponent_score_and_removes_ball() {
        let field_dimensions = FieldDimensions::SPL_2025;
        let config = AutoRefereeConfig::default();
        let mut game_state = SimulatorGameState::default();
        let mut auto_referee = AutoRefereeState::default();
        let mut ball = SimulatorBall {
            state: Some(SimulatedBall {
                position: point![-field_dimensions.length / 2.0 - 0.1, 0.0],
                velocity: vector![0.0, 0.0],
                field_side: Side::Left,
            }),
        };
        let mut rule = ScoredGoalRule;

        rule.apply(&mut auto_referee_context(
            SystemTime::UNIX_EPOCH + Duration::from_secs(1),
            &config,
            field_dimensions,
            &mut game_state,
            &mut auto_referee,
            &mut ball,
        ));

        assert_eq!(game_state.game_controller_state.hulks_team.score, 0);
        assert_eq!(game_state.game_controller_state.opponent_team.score, 1);
        assert_eq!(
            game_state.game_controller_state.game_state,
            GameState::Ready
        );
        assert_eq!(
            game_state.game_controller_state.kicking_team,
            Some(Team::Hulks)
        );
        assert!(ball.state.is_none());
    }

    #[test]
    fn auto_referee_config_defaults_match_hsl_timings() {
        let config = AutoRefereeConfig::default();

        assert_eq!(config.ready_duration, Duration::from_secs(45));
        assert_eq!(
            config.ready_stationary_short_circuit_duration,
            Some(Duration::from_secs(1))
        );
        assert_eq!(config.whistle_to_playing_delay, Duration::from_secs(3));
        assert_eq!(config.halftime_duration, Duration::from_secs(10 * 60));
        assert!(config.auto_whistle_in_set);
        assert!(config.finish_on_halftime_timeout);
    }

    #[test]
    fn scored_goal_at_ten_goal_difference_finishes_game() {
        let field_dimensions = FieldDimensions::SPL_2025;
        let config = AutoRefereeConfig::default();
        let mut game_state = SimulatorGameState::default();
        game_state.game_controller_state.opponent_team.score = 9;
        let mut auto_referee = AutoRefereeState::default();
        let mut ball = SimulatorBall {
            state: Some(SimulatedBall {
                position: point![-field_dimensions.length / 2.0 - 0.1, 0.0],
                velocity: vector![0.0, 0.0],
                field_side: Side::Left,
            }),
        };
        let mut rule = ScoredGoalRule;

        rule.apply(&mut auto_referee_context(
            SystemTime::UNIX_EPOCH + Duration::from_secs(1),
            &config,
            field_dimensions,
            &mut game_state,
            &mut auto_referee,
            &mut ball,
        ));

        assert_eq!(game_state.game_controller_state.opponent_team.score, 10);
        assert_eq!(
            game_state.game_controller_state.game_state,
            GameState::Finished
        );
        assert!(ball.state.is_none());
    }

    #[test]
    fn ready_transitions_to_set_after_ready_duration_and_places_ball() {
        let field_dimensions = FieldDimensions::SPL_2025;
        let config = transition_test_config();
        let mut game_state = SimulatorGameState::default();
        game_state.set_game_state(GameState::Ready, SystemTime::UNIX_EPOCH);
        let mut auto_referee = AutoRefereeState {
            restart_reason: Some(SimulatorRestartReason::DroppedBall),
            ..Default::default()
        };
        let mut ball = SimulatorBall::default();
        let mut rule = GameStateTransitionRule;

        rule.apply(&mut auto_referee_context(
            SystemTime::UNIX_EPOCH,
            &config,
            field_dimensions,
            &mut game_state,
            &mut auto_referee,
            &mut ball,
        ));

        assert_eq!(game_state.game_controller_state.game_state, GameState::Set);
        assert_eq!(
            ball.state.expect("ball should be placed").position,
            Point2::origin()
        );
    }

    #[test]
    fn stationary_robots_short_circuit_ready_to_set() {
        let field_dimensions = FieldDimensions::SPL_2025;
        let config = AutoRefereeConfig {
            ready_duration: Duration::from_secs(45),
            ready_stationary_short_circuit_duration: Some(Duration::from_secs(1)),
            ..Default::default()
        };
        let mut game_state = SimulatorGameState::default();
        game_state.set_game_state(GameState::Ready, SystemTime::UNIX_EPOCH);
        let mut auto_referee = AutoRefereeState {
            restart_reason: Some(SimulatorRestartReason::DroppedBall),
            ..Default::default()
        };
        let mut ball = SimulatorBall::default();
        let mut rule = GameStateTransitionRule;
        let robot_poses = BTreeMap::from([(robot_id(PlayerNumber::Three), Isometry2::identity())]);

        rule.apply(&mut auto_referee_context_with_robot_poses(
            SystemTime::UNIX_EPOCH,
            &config,
            field_dimensions,
            &mut game_state,
            &mut auto_referee,
            &mut ball,
            robot_poses.clone(),
        ));
        assert_eq!(
            game_state.game_controller_state.game_state,
            GameState::Ready
        );

        rule.apply(&mut auto_referee_context_with_robot_poses(
            SystemTime::UNIX_EPOCH + Duration::from_secs(1),
            &config,
            field_dimensions,
            &mut game_state,
            &mut auto_referee,
            &mut ball,
            robot_poses,
        ));

        assert_eq!(game_state.game_controller_state.game_state, GameState::Set);
        assert_eq!(
            ball.state.expect("ball should be placed").position,
            Point2::origin()
        );
    }

    #[test]
    fn moving_robot_prevents_ready_short_circuit() {
        let field_dimensions = FieldDimensions::SPL_2025;
        let config = AutoRefereeConfig {
            ready_duration: Duration::from_secs(45),
            ready_stationary_short_circuit_duration: Some(Duration::from_secs(1)),
            ..Default::default()
        };
        let mut game_state = SimulatorGameState::default();
        game_state.set_game_state(GameState::Ready, SystemTime::UNIX_EPOCH);
        let mut auto_referee = AutoRefereeState::default();
        let mut ball = SimulatorBall::default();
        let mut rule = GameStateTransitionRule;

        rule.apply(&mut auto_referee_context_with_robot_poses(
            SystemTime::UNIX_EPOCH,
            &config,
            field_dimensions,
            &mut game_state,
            &mut auto_referee,
            &mut ball,
            BTreeMap::from([(robot_id(PlayerNumber::Three), Isometry2::identity())]),
        ));
        rule.apply(&mut auto_referee_context_with_robot_poses(
            SystemTime::UNIX_EPOCH + Duration::from_secs(1),
            &config,
            field_dimensions,
            &mut game_state,
            &mut auto_referee,
            &mut ball,
            BTreeMap::from([(
                robot_id(PlayerNumber::Three),
                Isometry2::from_parts(vector![1.0, 0.0], 0.0),
            )]),
        ));

        assert_eq!(
            game_state.game_controller_state.game_state,
            GameState::Ready
        );
    }

    #[test]
    fn disabling_ready_short_circuit_keeps_ready_until_timeout() {
        let field_dimensions = FieldDimensions::SPL_2025;
        let config = AutoRefereeConfig {
            ready_duration: Duration::from_secs(45),
            ready_stationary_short_circuit_duration: None,
            ..Default::default()
        };
        let mut game_state = SimulatorGameState::default();
        game_state.set_game_state(GameState::Ready, SystemTime::UNIX_EPOCH);
        let mut auto_referee = AutoRefereeState::default();
        let mut ball = SimulatorBall::default();
        let mut rule = GameStateTransitionRule;
        let robot_poses = BTreeMap::from([(robot_id(PlayerNumber::Three), Isometry2::identity())]);

        rule.apply(&mut auto_referee_context_with_robot_poses(
            SystemTime::UNIX_EPOCH,
            &config,
            field_dimensions,
            &mut game_state,
            &mut auto_referee,
            &mut ball,
            robot_poses.clone(),
        ));
        rule.apply(&mut auto_referee_context_with_robot_poses(
            SystemTime::UNIX_EPOCH + Duration::from_secs(1),
            &config,
            field_dimensions,
            &mut game_state,
            &mut auto_referee,
            &mut ball,
            robot_poses,
        ));

        assert_eq!(
            game_state.game_controller_state.game_state,
            GameState::Ready
        );
    }

    #[test]
    fn set_transitions_to_playing_after_whistle_delay() {
        let field_dimensions = FieldDimensions::SPL_2025;
        let config = transition_test_config();
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1);
        let mut game_state = SimulatorGameState::default();
        game_state.set_game_state(GameState::Set, SystemTime::UNIX_EPOCH);
        let mut auto_referee = AutoRefereeState {
            playing_after_whistle_at: Some(now),
            ..Default::default()
        };
        let mut ball = SimulatorBall::default();
        let mut rule = GameStateTransitionRule;

        rule.apply(&mut auto_referee_context(
            now,
            &config,
            field_dimensions,
            &mut game_state,
            &mut auto_referee,
            &mut ball,
        ));

        assert_eq!(
            game_state.game_controller_state.game_state,
            GameState::Playing
        );
        assert_eq!(auto_referee.playing_after_whistle_at, None);
        assert_eq!(auto_referee.halftime_started_at, Some(now));
    }

    #[test]
    fn playing_finishes_after_halftime_duration_by_default() {
        let field_dimensions = FieldDimensions::SPL_2025;
        let config = AutoRefereeConfig {
            halftime_duration: Duration::ZERO,
            ..Default::default()
        };
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1);
        let mut game_state = SimulatorGameState::default();
        let mut auto_referee = AutoRefereeState {
            halftime_started_at: Some(now),
            ..Default::default()
        };
        let mut ball = SimulatorBall::default();
        let mut rule = HalftimeTimeoutRule;

        rule.apply(&mut auto_referee_context(
            now,
            &config,
            field_dimensions,
            &mut game_state,
            &mut auto_referee,
            &mut ball,
        ));

        assert_eq!(
            game_state.game_controller_state.game_state,
            GameState::Finished
        );
    }

    #[test]
    fn disabling_halftime_timeout_prevents_finish() {
        let field_dimensions = FieldDimensions::SPL_2025;
        let config = AutoRefereeConfig {
            halftime_duration: Duration::ZERO,
            finish_on_halftime_timeout: false,
            ..Default::default()
        };
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1);
        let mut game_state = SimulatorGameState::default();
        let mut auto_referee = AutoRefereeState {
            halftime_started_at: Some(now),
            ..Default::default()
        };
        let mut ball = SimulatorBall::default();
        let mut rule = HalftimeTimeoutRule;

        rule.apply(&mut auto_referee_context(
            now,
            &config,
            field_dimensions,
            &mut game_state,
            &mut auto_referee,
            &mut ball,
        ));

        assert_eq!(
            game_state.game_controller_state.game_state,
            GameState::Playing
        );
    }

    #[test]
    fn brief_stop_and_timeout_commands_sync_filtered_state() {
        let field_dimensions = FieldDimensions::SPL_2025;
        let config = AutoRefereeConfig::default();
        let mut game_state = SimulatorGameState::default();
        let mut auto_referee = AutoRefereeState::default();
        let mut ball = SimulatorBall::default();
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1);

        let mut context = auto_referee_context(
            now,
            &config,
            field_dimensions,
            &mut game_state,
            &mut auto_referee,
            &mut ball,
        );
        apply_referee_command(SimulatorRefereeCommand::BriefStop, &mut context);
        assert_eq!(
            context
                .game_state
                .filtered_game_controller_state
                .as_ref()
                .expect("filtered state should exist")
                .game_state,
            FilteredGameState::Stop
        );

        apply_referee_command(SimulatorRefereeCommand::Resume, &mut context);
        apply_referee_command(SimulatorRefereeCommand::SetTimeout(true), &mut context);
        assert_eq!(
            context.game_state.game_controller_state.game_phase,
            GamePhase::Timeout
        );
        assert_eq!(
            context
                .game_state
                .filtered_game_controller_state
                .as_ref()
                .expect("filtered state should exist")
                .game_phase,
            GamePhase::Timeout
        );
    }
}
