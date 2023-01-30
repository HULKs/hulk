use color_eyre::eyre::Context;

#[derive(
    Clone,
    Default,
    serde :: Deserialize,
    serde :: Serialize,
    serialize_hierarchy :: SerializeHierarchy,
)]
pub struct Database {
    pub main_outputs: structs::control::MainOutputs,
    pub additional_outputs: structs::control::AdditionalOutputs,
    pub simulator_database: SimulatorDatabase,
}

#[derive(
    Clone,
    Default,
    serde :: Deserialize,
    serde :: Serialize,
    serialize_hierarchy :: SerializeHierarchy,
)]
pub struct SimulatorDatabase {}

pub struct BehaviorCycler<Interface> {
    hardware_interface: std::sync::Arc<Interface>,
    own_changed: std::sync::Arc<tokio::sync::Notify>,
    configuration_reader: framework::Reader<structs::Configuration>,
    // historic_databases: framework::HistoricDatabases<structs::control::MainOutputs>,
    perception_databases: framework::PerceptionDatabases,

    // sensor_data_receiver: control::sensor_data_receiver::SensorDataReceiver,
    // sole_pressure_filter: control::sole_pressure_filter::SolePressureFilter,
    // ground_contact_detector: control::ground_contact_detector::GroundContactDetector,
    // fall_state_estimation: control::fall_state_estimation::FallStateEstimation,
    // sonar_filter: control::sonar_filter::SonarFilter,
    // button_filter: control::button_filter::ButtonFilter,
    // support_foot_estimation: control::support_foot_estimation::SupportFootEstimation,
    // orientation_filter: control::orientation_filter::OrientationFilter,
    // kinematics_provider: control::kinematics_provider::KinematicsProvider,
    // ground_provider: control::ground_provider::GroundProvider,
    // center_of_mass_provider: control::center_of_mass_provider::CenterOfMassProvider,
    // whistle_filter: control::whistle_filter::WhistleFilter,
    // camera_matrix_calculator: control::camera_matrix_calculator::CameraMatrixCalculator,
    // odometry: control::odometry::Odometry,
    // limb_projector: control::limb_projector::LimbProjector,
    // ball_filter: control::ball_filter::BallFilter,
    // game_controller_filter: control::game_controller_filter::GameControllerFilter,
    // game_state_filter: control::game_state_filter::GameStateFilter,
    // primary_state_filter: control::primary_state_filter::PrimaryStateFilter,
    // localization: control::localization::Localization,
    // penalty_shot_direction_estimation:
    //     control::penalty_shot_direction_estimation::PenaltyShotDirectionEstimation,
    role_assignment: control::role_assignment::RoleAssignment,
    // obstacle_filter: control::obstacle_filter::ObstacleFilter,
    world_state_composer: control::world_state_composer::WorldStateComposer,
    behavior: control::behavior::node::Behavior,
    // step_planner: control::motion::step_planner::StepPlanner,
    // look_at: control::motion::look_at::LookAt,
    // motion_selector: control::motion::motion_selector::MotionSelector,
    // stand_up_back: control::motion::stand_up_back::StandUpBack,
    // stand_up_front: control::motion::stand_up_front::StandUpFront,
    // walk_manager: control::motion::walk_manager::WalkManager,
    // fall_protector: control::motion::fall_protector::FallProtector,
    // jump_left: control::motion::jump_left::JumpLeft,
    // jump_right: control::motion::jump_right::JumpRight,
    // sit_down: control::motion::sit_down::SitDown,
    // led_status: control::led_status::LedStatus,
    // arms_up_squat: control::motion::arms_up_squat::ArmsUpSquat,
    // look_around: control::motion::look_around::LookAround,
    // head_motion: control::motion::head_motion::HeadMotion,
    // walking_engine: control::motion::walking_engine::WalkingEngine,
    // dispatching_interpolator: control::motion::dispatching_interpolator::DispatchingInterpolator,
    // joint_command_sender: control::motion::joint_command_sender::JointCommandSender,
}

impl<Interface> BehaviorCycler<Interface>
where
    Interface: types::hardware::Interface + std::marker::Send + std::marker::Sync + 'static,
{
    pub fn new(
        hardware_interface: std::sync::Arc<Interface>,
        own_changed: std::sync::Arc<tokio::sync::Notify>,
        configuration_reader: framework::Reader<structs::Configuration>,
    ) -> color_eyre::Result<Self> {
        let configuration = configuration_reader.next().clone();

        let role_assignment = control::role_assignment::RoleAssignment::new(
            control::role_assignment::CreationContext {
                forced_role: configuration.role_assignment.forced_role.as_ref(),
                player_number: &configuration.player_number,
                spl_network: &configuration.spl_network,
            },
        )
        .wrap_err("failed to create node `RoleAssignment`")?;
        let world_state_composer = control::world_state_composer::WorldStateComposer::new(
            control::world_state_composer::CreationContext {
                player_number: &configuration.player_number,
            },
        )
        .wrap_err("failed to create node `WorldStateComposer`")?;
        let behavior =
            control::behavior::node::Behavior::new(control::behavior::node::CreationContext {
                behavior: &configuration.behavior,
                field_dimensions: &configuration.field_dimensions,
                lost_ball_parameters: &configuration.behavior.lost_ball,
            })
            .wrap_err("failed to create node `Behavior`")?;

        Ok(Self {
            hardware_interface,
            own_changed,
            configuration_reader,
            perception_databases: Default::default(),

            role_assignment,
            world_state_composer,
            behavior,
        })
    }

    pub fn cycle(&mut self, own_database_reference: &mut Database) -> color_eyre::Result<()> {
        use color_eyre::eyre::WrapErr;
        {
            // let now = self.hardware_interface.get_now();
            {
                let configuration = self.configuration_reader.next();
                {
                    let main_outputs = self
                        .role_assignment
                        .cycle(control::role_assignment::CycleContext {
                            ball_position: own_database_reference
                                .main_outputs
                                .ball_position
                                .as_ref(),
                            fall_state: &own_database_reference.main_outputs.fall_state,
                            game_controller_state: own_database_reference
                                .main_outputs
                                .game_controller_state
                                .as_ref(),
                            primary_state: &own_database_reference.main_outputs.primary_state,
                            robot_to_field: own_database_reference
                                .main_outputs
                                .robot_to_field
                                .as_ref(),
                            cycle_time: &own_database_reference.main_outputs.cycle_time,
                            forced_role: configuration.role_assignment.forced_role.as_ref(),
                            player_number: &configuration.player_number,
                            spl_network: &configuration.spl_network,
                            network_message: framework::PerceptionInput {
                                persistent: self
                                    .perception_databases
                                    .persistent()
                                    .map(|(system_time, databases)| {
                                        (
                                            *system_time,
                                            databases
                                                .spl_network
                                                .iter()
                                                .map(|database| &database.message)
                                                .collect(),
                                        )
                                    })
                                    .collect(),
                                temporary: self
                                    .perception_databases
                                    .temporary()
                                    .map(|(system_time, databases)| {
                                        (
                                            *system_time,
                                            databases
                                                .spl_network
                                                .iter()
                                                .map(|database| &database.message)
                                                .collect(),
                                        )
                                    })
                                    .collect(),
                            },
                            hardware: &self.hardware_interface,
                        })
                        .wrap_err("failed to execute cycle of node `RoleAssignment`")?;
                    own_database_reference.main_outputs.team_ball = main_outputs.team_ball.value;
                    own_database_reference.main_outputs.network_robot_obstacles =
                        main_outputs.network_robot_obstacles.value;
                    own_database_reference.main_outputs.role = main_outputs.role.value;
                }
                {
                    let main_outputs = self
                        .world_state_composer
                        .cycle(control::world_state_composer::CycleContext {
                            ball_position: own_database_reference
                                .main_outputs
                                .ball_position
                                .as_ref(),
                            filtered_game_state: own_database_reference
                                .main_outputs
                                .filtered_game_state
                                .as_ref(),
                            game_controller_state: own_database_reference
                                .main_outputs
                                .game_controller_state
                                .as_ref(),
                            penalty_shot_direction: own_database_reference
                                .main_outputs
                                .penalty_shot_direction
                                .as_ref(),
                            robot_to_field: own_database_reference
                                .main_outputs
                                .robot_to_field
                                .as_ref(),
                            team_ball: own_database_reference.main_outputs.team_ball.as_ref(),
                            player_number: &configuration.player_number,
                            fall_state: &own_database_reference.main_outputs.fall_state,
                            has_ground_contact: &own_database_reference
                                .main_outputs
                                .has_ground_contact,
                            obstacles: &own_database_reference.main_outputs.obstacles,
                            primary_state: &own_database_reference.main_outputs.primary_state,
                            role: &own_database_reference.main_outputs.role,
                        })
                        .wrap_err("failed to execute cycle of node `WorldStateComposer`")?;
                    own_database_reference.main_outputs.world_state =
                        main_outputs.world_state.value;
                }
                {
                    let main_outputs = self
                        .behavior
                        .cycle(control::behavior::node::CycleContext {
                            kick_decisions: framework::AdditionalOutput::new(
                                true,
                                &mut own_database_reference.additional_outputs.kick_decisions,
                            ),
                            kick_targets: framework::AdditionalOutput::new(
                                true,
                                &mut own_database_reference.additional_outputs.kick_targets,
                            ),
                            path_obstacles: framework::AdditionalOutput::new(
                                true,
                                &mut own_database_reference.additional_outputs.path_obstacles,
                            ),
                            world_state: &own_database_reference.main_outputs.world_state,
                            configuration: &configuration.behavior,
                            field_dimensions: &configuration.field_dimensions,
                            lost_ball_parameters: &configuration.behavior.lost_ball,
                        })
                        .wrap_err("failed to execute cycle of node `Behavior`")?;
                    own_database_reference.main_outputs.motion_command =
                        main_outputs.motion_command.value;
                }
            }
        }
        self.own_changed.notify_one();
        Ok(())
    }
}
