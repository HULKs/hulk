use std::fmt::{Display, Formatter, Result};
use std::{collections::BTreeMap, time::SystemTime};

use bevy::prelude::*;
use booster::FallDownState;
use coordinate_systems::{Field, Ground, World};
use hsl_network_messages::PlayerNumber;
use linear_algebra::{Isometry2, Orientation2, Point2};
use serde::Serialize;
use types::path::traits::EndPoints;
use types::{
    field_dimensions::FieldDimensions, motion_command::MotionCommand, primary_state::PrimaryState,
    rule_obstacles::RuleObstacle,
};

use crate::behavior_tree_simulator::{
    RobotFrame, SimulatedBall, SimulationConfig, SimulatorBall, SimulatorFailure,
    SimulatorFallDownState, SimulatorFieldDimensions, SimulatorGroundToWorld,
    SimulatorPrimaryState, SimulatorRobot, SimulatorRobotFrames, SimulatorRobotId,
    SimulatorRuleObstacles, SimulatorScenarioResult,
};
use crate::timeline::robot_snapshots_from_query;

#[derive(Resource, Default)]
pub struct SimulatorInvariantChecks(pub Vec<Box<dyn InvariantCheck>>);

#[derive(Resource, Clone, Debug, Default)]
pub struct SimulatorCurrentInvariantViolations(pub Vec<InvariantViolation>);

pub const BEHAVIOR_TICK_ERROR_CHECK_NAME: &str = "behavior_tick_error";

#[derive(Clone, Copy, Debug, Serialize)]
pub struct RobotSnapshot {
    pub id: SimulatorRobotId,
    pub player_number: PlayerNumber,
    pub ground_to_world: Isometry2<Ground, World>,
    pub head_yaw: Orientation2<Ground>,
    pub primary_state: PrimaryState,
    pub fall_down_state: Option<FallDownState>,
}

#[derive(Clone, Debug)]
pub struct SimulationSnapshot {
    pub now: SystemTime,
    pub ball: Option<SimulatedBall>,
    pub robots: BTreeMap<SimulatorRobotId, RobotSnapshot>,
    pub robot_frames: BTreeMap<SimulatorRobotId, RobotFrame>,
    pub field_dimensions: FieldDimensions,
    pub rule_obstacles: Vec<RuleObstacle>,
    pub config: SimulationConfig,
}

#[derive(Clone, Debug, Serialize)]
pub struct InvariantViolation {
    pub check_name: &'static str,
    pub player_number: Option<PlayerNumber>,
    pub message: String,
    pub severity: InvariantSeverity,
}

impl Display for InvariantViolation {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result {
        write!(
            formatter,
            "{} {:?}: {}",
            self.check_name, self.player_number, self.message
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum InvariantSeverity {
    Warning,
    Error,
}

pub trait InvariantCheck: Send + Sync {
    fn check(&mut self, snapshot: &SimulationSnapshot) -> Vec<InvariantViolation>;
}

pub fn default_invariant_checks() -> Vec<Box<dyn InvariantCheck>> {
    vec![
        Box::new(RuleObstacleWalkCheck),
        Box::new(FieldBoundaryWalkCheck),
    ]
}

pub struct RuleObstacleWalkCheck;

impl InvariantCheck for RuleObstacleWalkCheck {
    fn check(&mut self, snapshot: &SimulationSnapshot) -> Vec<InvariantViolation> {
        let mut violations = Vec::new();
        for (robot_id, frame) in &snapshot.robot_frames {
            let Some(target) = motion_target_in_field(frame) else {
                continue;
            };

            for obstacle in &frame.world_state.rule_obstacles {
                if obstacle.contains(target) {
                    violations.push(InvariantViolation {
                        check_name: "rule_obstacle_walk",
                        player_number: Some(robot_id.player_number),
                        message: format!(
                            "robot {robot_id} plans to walk into a known rule obstacle"
                        ),
                        severity: InvariantSeverity::Error,
                    });
                    break;
                }
            }
        }
        violations
    }
}

pub struct FieldBoundaryWalkCheck;

impl InvariantCheck for FieldBoundaryWalkCheck {
    fn check(&mut self, snapshot: &SimulationSnapshot) -> Vec<InvariantViolation> {
        let mut violations = Vec::new();
        for (robot_id, frame) in &snapshot.robot_frames {
            let Some(target) = motion_target_in_field(frame) else {
                continue;
            };

            if !is_inside_field_with_border_margin(target, snapshot.field_dimensions) {
                violations.push(InvariantViolation {
                    check_name: "field_boundary_walk",
                    player_number: Some(robot_id.player_number),
                    message: format!("robot {robot_id} plans to walk outside the known field"),
                    severity: InvariantSeverity::Error,
                });
            }
        }
        violations
    }
}

pub fn run_invariant_checks(
    clock: Res<crate::behavior_tree_simulator::SimulatorClock>,
    ball: Res<SimulatorBall>,
    field_dimensions: Res<SimulatorFieldDimensions>,
    rule_obstacles: Res<SimulatorRuleObstacles>,
    config: Res<SimulationConfig>,
    robot_frames: Res<SimulatorRobotFrames>,
    mut invariant_checks: ResMut<SimulatorInvariantChecks>,
    mut current_violations: ResMut<SimulatorCurrentInvariantViolations>,
    mut scenario_result: ResMut<SimulatorScenarioResult>,
    robots: Query<(
        &SimulatorRobot,
        &SimulatorGroundToWorld,
        &crate::behavior_tree_simulator::SimulatorHeadYaw,
        &SimulatorPrimaryState,
        &SimulatorFallDownState,
    )>,
) {
    current_violations
        .0
        .retain(|violation| violation.check_name == BEHAVIOR_TICK_ERROR_CHECK_NAME);

    let snapshot = SimulationSnapshot {
        now: clock.now,
        ball: ball.state,
        robots: robot_snapshots_from_query(&robots),
        robot_frames: robot_frames.0.clone(),
        field_dimensions: field_dimensions.0,
        rule_obstacles: rule_obstacles.obstacles.clone(),
        config: config.clone(),
    };

    for check in &mut invariant_checks.0 {
        current_violations.0.extend(check.check(&snapshot));
    }

    if !current_violations.0.is_empty() {
        scenario_result.failed = true;
        scenario_result.failures.extend(
            current_violations
                .0
                .iter()
                .cloned()
                .map(SimulatorFailure::InvariantViolation),
        );
    }
}

fn is_inside_field_with_border_margin(
    target: Point2<Field>,
    field_dimensions: FieldDimensions,
) -> bool {
    let x_max = field_dimensions.length / 2.0 + field_dimensions.border_strip_width;
    let y_max = field_dimensions.width / 2.0 + field_dimensions.border_strip_width;
    target.x().abs() < x_max && target.y().abs() < y_max
}

fn motion_target_in_field(frame: &RobotFrame) -> Option<Point2<Field>> {
    let MotionCommand::Walk { path, .. } = &frame.motion_command else {
        return None;
    };
    let ground_to_field = frame.world_state.robot.ground_to_field?;
    Some(ground_to_field * path.end_point())
}
