use std::{collections::BTreeMap, sync::Arc, time::SystemTime};

use color_eyre::{eyre::WrapErr, Result};
use control::{
    active_vision::{self, ActiveVision},
    ball_state_composer::{self, BallStateComposer},
    behavior::node::{self, Behavior},
    role_assignment::{self, RoleAssignment},
    world_state_composer::{self, WorldStateComposer},
};
use cyclers::control::Database;
use framework::{AdditionalOutput, PerceptionInput};
use structs::Configuration;
use tokio::sync::Notify;
use types::{hardware, messages::IncomingMessage};

pub struct BehaviorCycler<Interface> {
    hardware_interface: Arc<Interface>,
    own_changed: Arc<Notify>,
    role_assignment: RoleAssignment,
    ball_state_composer: BallStateComposer,
    active_vision: ActiveVision,
    world_state_composer: WorldStateComposer,
    behavior: Behavior,
}

impl<Interface> BehaviorCycler<Interface>
where
    Interface: hardware::Interface,
{
    pub fn new(
        hardware_interface: Arc<Interface>,
        own_changed: Arc<Notify>,
        configuration: &Configuration,
    ) -> Result<Self> {
        let role_assignment = RoleAssignment::new(role_assignment::CreationContext {
            forced_role: configuration.role_assignment.forced_role.as_ref(),
            player_number: &configuration.player_number,
            spl_network: &configuration.spl_network,
        })
        .wrap_err("failed to create node `RoleAssignment`")?;
        let ball_state_composer = BallStateComposer::new(ball_state_composer::CreationContext {})
            .wrap_err("failed to create node `BallStateComposer`")?;
        let active_vision = ActiveVision::new(active_vision::CreationContext {
            field_dimensions: &configuration.field_dimensions,
        })
        .wrap_err("failed to create node `ActiveVision`")?;
        let world_state_composer = WorldStateComposer::new(world_state_composer::CreationContext {
            player_number: &configuration.player_number,
        })
        .wrap_err("failed to create node `WorldStateComposer`")?;
        let behavior = Behavior::new(node::CreationContext {
            behavior: &configuration.behavior,
            field_dimensions: &configuration.field_dimensions,
            lost_ball_parameters: &configuration.behavior.lost_ball,
        })
        .wrap_err("failed to create node `Behavior`")?;

        Ok(Self {
            hardware_interface,
            own_changed,

            role_assignment,
            ball_state_composer,
            active_vision,
            world_state_composer,
            behavior,
        })
    }

    pub fn cycle(
        &mut self,
        own_database: &mut Database,
        configuration: &Configuration,
        incoming_messages: BTreeMap<SystemTime, Vec<&IncomingMessage>>,
    ) -> Result<()> {
        {
            let main_outputs = self
                .role_assignment
                .cycle(role_assignment::CycleContext {
                    ball_position: own_database.main_outputs.ball_position.as_ref(),
                    fall_state: &own_database.main_outputs.fall_state,
                    game_controller_state: own_database.main_outputs.game_controller_state.as_ref(),
                    primary_state: &own_database.main_outputs.primary_state,
                    robot_to_field: own_database.main_outputs.robot_to_field.as_ref(),
                    cycle_time: &own_database.main_outputs.cycle_time,
                    field_dimensions: &configuration.field_dimensions,
                    forced_role: configuration.role_assignment.forced_role.as_ref(),
                    initial_poses: &configuration.localization.initial_poses,
                    optional_roles: &configuration.behavior.optional_roles,
                    player_number: &configuration.player_number,
                    spl_network: &configuration.spl_network,
                    network_message: PerceptionInput {
                        persistent: incoming_messages,
                        temporary: Default::default(),
                    },
                    hardware: &self.hardware_interface,
                })
                .wrap_err("failed to execute cycle of node `RoleAssignment`")?;
            own_database.main_outputs.team_ball = main_outputs.team_ball.value;
            own_database.main_outputs.network_robot_obstacles =
                main_outputs.network_robot_obstacles.value;
            own_database.main_outputs.role = main_outputs.role.value;
        }
        {
            let main_outputs = self
                .ball_state_composer
                .cycle(ball_state_composer::CycleContext {
                    ball_position: own_database.main_outputs.ball_position.as_ref(),
                    penalty_shot_direction: own_database
                        .main_outputs
                        .penalty_shot_direction
                        .as_ref(),
                    robot_to_field: own_database.main_outputs.robot_to_field.as_ref(),
                    team_ball: own_database.main_outputs.team_ball.as_ref(),
                    primary_state: &own_database.main_outputs.primary_state,
                    field_dimensions: &configuration.field_dimensions,
                    game_controller_state: own_database.main_outputs.game_controller_state.as_ref(),
                })
                .wrap_err("failed to execute cycle of node `BallStateComposer`")?;
            own_database.main_outputs.ball_state = main_outputs.ball_state.value;
            own_database.main_outputs.rule_ball_state = main_outputs.rule_ball_state.value;
        }
        {
            let main_outputs = self
                .active_vision
                .cycle(active_vision::CycleContext {
                    ball: own_database.main_outputs.ball_state.as_ref(),
                    rule_ball: own_database.main_outputs.ball_state.as_ref(),
                    cycle_time: &own_database.main_outputs.cycle_time,
                    obstacles: &own_database.main_outputs.obstacles,
                    parameters: &configuration.behavior.look_action,
                    robot_to_field: own_database.main_outputs.robot_to_field.as_ref(),
                })
                .wrap_err("failed to execute cycle of node `ActiveVision`")?;
            own_database.main_outputs.position_of_interest =
                main_outputs.position_of_interest.value;
        }
        {
            let main_outputs = self
                .world_state_composer
                .cycle(world_state_composer::CycleContext {
                    ball: own_database.main_outputs.ball_state.as_ref(),
                    filtered_game_state: own_database.main_outputs.filtered_game_state.as_ref(),
                    game_controller_state: own_database.main_outputs.game_controller_state.as_ref(),
                    penalty_shot_direction: own_database
                        .main_outputs
                        .penalty_shot_direction
                        .as_ref(),
                    robot_to_field: own_database.main_outputs.robot_to_field.as_ref(),
                    player_number: &configuration.player_number,
                    fall_state: &own_database.main_outputs.fall_state,
                    has_ground_contact: &own_database.main_outputs.has_ground_contact,
                    obstacles: &own_database.main_outputs.obstacles,
                    primary_state: &own_database.main_outputs.primary_state,
                    role: &own_database.main_outputs.role,
                    position_of_interest: &own_database.main_outputs.position_of_interest,
                    rule_ball: own_database.main_outputs.rule_ball_state.as_ref(),
                    rule_obstacles: &own_database.main_outputs.rule_obstacles,
                })
                .wrap_err("failed to execute cycle of node `WorldStateComposer`")?;
            own_database.main_outputs.world_state = main_outputs.world_state.value;
        }
        {
            let main_outputs = self
                .behavior
                .cycle(node::CycleContext {
                    kick_decisions: AdditionalOutput::new(
                        true,
                        &mut own_database.additional_outputs.kick_decisions,
                    ),
                    best_kick_decision: AdditionalOutput::new(
                        true,
                        &mut own_database.additional_outputs.best_kick_decision,
                    ),
                    kick_targets: AdditionalOutput::new(
                        true,
                        &mut own_database.additional_outputs.kick_targets,
                    ),
                    path_obstacles: AdditionalOutput::new(
                        true,
                        &mut own_database.additional_outputs.path_obstacles,
                    ),
                    world_state: &own_database.main_outputs.world_state,
                    cycle_time: &own_database.main_outputs.cycle_time,
                    configuration: &configuration.behavior,
                    field_dimensions: &configuration.field_dimensions,
                    lost_ball_parameters: &configuration.behavior.lost_ball,
                    has_ground_contact: &true,
                })
                .wrap_err("failed to execute cycle of node `Behavior`")?;
            own_database.main_outputs.motion_command = main_outputs.motion_command.value;
        }
        self.own_changed.notify_one();
        Ok(())
    }
}
