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

pub struct Cycler<Interface> {
    instance: control::CyclerInstance,
    hardware_interface: std::sync::Arc<Interface>,
    own_writer: framework::Writer<Database>,
    own_changed: std::sync::Arc<tokio::sync::Notify>,
    own_subscribed_outputs_reader: framework::Reader<std::collections::HashSet<String>>,
    configuration_reader: framework::Reader<structs::Configuration>,
    historic_databases: framework::HistoricDatabases<structs::control::MainOutputs>,
    perception_databases: framework::PerceptionDatabases,
    persistent_state: structs::control::PersistentState,

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
    ball_filter: control::ball_filter::BallFilter,
    game_controller_filter: control::game_controller_filter::GameControllerFilter,
    game_state_filter: control::game_state_filter::GameStateFilter,
    primary_state_filter: control::primary_state_filter::PrimaryStateFilter,
    // localization: control::localization::Localization,
    penalty_shot_direction_estimation:
        control::penalty_shot_direction_estimation::PenaltyShotDirectionEstimation,
    role_assignment: control::role_assignment::RoleAssignment,
    // obstacle_filter: control::obstacle_filter::ObstacleFilter,
    world_state_composer: control::world_state_composer::WorldStateComposer,
    behavior: control::behavior::node::Behavior,
    // step_planner: control::motion::step_planner::StepPlanner,
    // look_at: control::motion::look_at::LookAt,
    motion_selector: control::motion::motion_selector::MotionSelector,
    // stand_up_back: control::motion::stand_up_back::StandUpBack,
    // stand_up_front: control::motion::stand_up_front::StandUpFront,
    walk_manager: control::motion::walk_manager::WalkManager,
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

impl<Interface> Cycler<Interface>
where
    Interface: types::hardware::Interface + std::marker::Send + std::marker::Sync + 'static,
{
    pub fn new(
        instance: control::CyclerInstance,
        hardware_interface: std::sync::Arc<Interface>,
        own_writer: framework::Writer<Database>,
        own_changed: std::sync::Arc<tokio::sync::Notify>,
        own_subscribed_outputs_reader: framework::Reader<std::collections::HashSet<String>>,
        configuration_reader: framework::Reader<structs::Configuration>,
    ) -> color_eyre::Result<Self> {
        let configuration = configuration_reader.next().clone();
        let mut persistent_state = structs::control::PersistentState::default();

        let ball_filter =
            control::ball_filter::BallFilter::new(control::ball_filter::CreationContext {
                field_dimensions: &configuration.field_dimensions,
                hidden_validity_exponential_decay_factor: &configuration
                    .ball_filter
                    .hidden_validity_exponential_decay_factor,
                hypothesis_merge_distance: &configuration.ball_filter.hypothesis_merge_distance,
                hypothesis_timeout: &configuration.ball_filter.hypothesis_timeout,
                initial_covariance: &configuration.ball_filter.initial_covariance,
                measurement_matching_distance: &configuration
                    .ball_filter
                    .measurement_matching_distance,
                measurement_noise: &configuration.ball_filter.measurement_noise,
                process_noise: &configuration.ball_filter.process_noise,
                validity_discard_threshold: &configuration.ball_filter.validity_discard_threshold,
                visible_validity_exponential_decay_factor: &configuration
                    .ball_filter
                    .visible_validity_exponential_decay_factor,
            })
            .wrap_err("failed to create node `BallFilter`")?;
        let game_controller_filter = control::game_controller_filter::GameControllerFilter::new(
            control::game_controller_filter::CreationContext {},
        )
        .wrap_err("failed to create node `GameControllerFilter`")?;
        let game_state_filter = control::game_state_filter::GameStateFilter::new(
            control::game_state_filter::CreationContext {
                config: &configuration.game_state_filter,
                field_dimensions: &configuration.field_dimensions,
                player_number: &configuration.player_number,
                robot_to_field: &mut persistent_state.robot_to_field,
            },
        )
        .wrap_err("failed to create node `GameStateFilter`")?;
        let primary_state_filter = control::primary_state_filter::PrimaryStateFilter::new(
            control::primary_state_filter::CreationContext {
                player_number: &configuration.player_number,
            },
        )
        .wrap_err("failed to create node `PrimaryStateFilter`")?;
        let penalty_shot_direction_estimation =
            control::penalty_shot_direction_estimation::PenaltyShotDirectionEstimation::new(
                control::penalty_shot_direction_estimation::CreationContext {
                    field_dimensions: &configuration.field_dimensions,
                    moving_distance_threshold: &configuration
                        .penalty_shot_direction_estimation
                        .moving_distance_threshold,
                },
            )
            .wrap_err("failed to create node `PenaltyShotDirectionEstimation`")?;
        let role_assignment = control::role_assignment::RoleAssignment::new(
            control::role_assignment::CreationContext {
                field_dimensions: &configuration.field_dimensions,
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
        let motion_selector = control::motion::motion_selector::MotionSelector::new(
            control::motion::motion_selector::CreationContext {
                motion_safe_exits: &mut persistent_state.motion_safe_exits,
            },
        )
        .wrap_err("failed to create node `MotionSelector`")?;
        let walk_manager = control::motion::walk_manager::WalkManager::new(
            control::motion::walk_manager::CreationContext {},
        )
        .wrap_err("failed to create node `WalkManager`")?;

        Ok(Self {
            instance,
            hardware_interface,
            own_writer,
            own_changed,
            own_subscribed_outputs_reader,
            configuration_reader,
            historic_databases: Default::default(),
            perception_databases: Default::default(),
            persistent_state,
            ball_filter,
            game_controller_filter,
            game_state_filter,
            primary_state_filter,
            penalty_shot_direction_estimation,
            role_assignment,
            world_state_composer,
            behavior,
            motion_selector,
            walk_manager,
        })
    }

    pub fn cycle(&mut self) -> color_eyre::Result<()> {
        use color_eyre::eyre::WrapErr;
        {
            let mut own_database = self.own_writer.next();
            let own_database_reference = {
                use std::ops::DerefMut;
                own_database.deref_mut()
            };
            let now = self.hardware_interface.get_now();
            {
                let own_subscribed_outputs = self.own_subscribed_outputs_reader.next();
                let configuration = self.configuration_reader.next();
                {
                    let main_outputs = self
                        .game_controller_filter
                        .cycle(control::game_controller_filter::CycleContext {
                            sensor_data: &own_database_reference.main_outputs.sensor_data,
                            cycle_time: &own_database_reference.main_outputs.cycle_time,
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
                        })
                        .wrap_err("failed to execute cycle of node `GameControllerFilter`")?;
                    own_database_reference.main_outputs.game_controller_state =
                        main_outputs.game_controller_state.value;
                }
                if own_database_reference
                    .main_outputs
                    .camera_matrices
                    .as_ref()
                    .is_some()
                {
                    let main_outputs = self
                        .ball_filter
                        .cycle(control::ball_filter::CycleContext {
                            ball_filter_hypotheses: framework::AdditionalOutput::new(
                                own_subscribed_outputs.iter().any(|subscribed_output| {
                                    framework::should_be_filled(
                                        subscribed_output,
                                        "additional_outputs.ball_filter_hypotheses",
                                    )
                                }),
                                &mut own_database_reference
                                    .additional_outputs
                                    .ball_filter_hypotheses,
                            ),
                            filtered_balls_in_image_bottom: framework::AdditionalOutput::new(
                                own_subscribed_outputs.iter().any(|subscribed_output| {
                                    framework::should_be_filled(
                                        subscribed_output,
                                        "additional_outputs.filtered_balls_in_image_bottom",
                                    )
                                }),
                                &mut own_database_reference
                                    .additional_outputs
                                    .filtered_balls_in_image_bottom,
                            ),
                            filtered_balls_in_image_top: framework::AdditionalOutput::new(
                                own_subscribed_outputs.iter().any(|subscribed_output| {
                                    framework::should_be_filled(
                                        subscribed_output,
                                        "additional_outputs.filtered_balls_in_image_top",
                                    )
                                }),
                                &mut own_database_reference
                                    .additional_outputs
                                    .filtered_balls_in_image_top,
                            ),
                            current_odometry_to_last_odometry: [(
                                now,
                                own_database_reference
                                    .main_outputs
                                    .current_odometry_to_last_odometry
                                    .as_ref(),
                            )]
                            .into_iter()
                            .chain(self.historic_databases.databases.iter().map(
                                |(system_time, database)| {
                                    (
                                        *system_time,
                                        database.current_odometry_to_last_odometry.as_ref(),
                                    )
                                },
                            ))
                            .collect::<std::collections::BTreeMap<_, _>>()
                            .into(),
                            historic_camera_matrices: [(
                                now,
                                own_database_reference.main_outputs.camera_matrices.as_ref(),
                            )]
                            .into_iter()
                            .chain(self.historic_databases.databases.iter().map(
                                |(system_time, database)| {
                                    (*system_time, database.camera_matrices.as_ref())
                                },
                            ))
                            .collect::<std::collections::BTreeMap<_, _>>()
                            .into(),
                            projected_limbs: [(
                                now,
                                own_database_reference.main_outputs.projected_limbs.as_ref(),
                            )]
                            .into_iter()
                            .chain(self.historic_databases.databases.iter().map(
                                |(system_time, database)| {
                                    (*system_time, database.projected_limbs.as_ref())
                                },
                            ))
                            .collect::<std::collections::BTreeMap<_, _>>()
                            .into(),
                            camera_matrices: own_database_reference
                                .main_outputs
                                .camera_matrices
                                .as_ref()
                                .unwrap(),
                            sensor_data: &own_database_reference.main_outputs.sensor_data,
                            cycle_time: &own_database_reference.main_outputs.cycle_time,
                            field_dimensions: &configuration.field_dimensions,
                            hidden_validity_exponential_decay_factor: &configuration
                                .ball_filter
                                .hidden_validity_exponential_decay_factor,
                            hypothesis_merge_distance: &configuration
                                .ball_filter
                                .hypothesis_merge_distance,
                            hypothesis_timeout: &configuration.ball_filter.hypothesis_timeout,
                            initial_covariance: &configuration.ball_filter.initial_covariance,
                            measurement_matching_distance: &configuration
                                .ball_filter
                                .measurement_matching_distance,
                            measurement_noise: &configuration.ball_filter.measurement_noise,
                            process_noise: &configuration.ball_filter.process_noise,
                            validity_discard_threshold: &configuration
                                .ball_filter
                                .validity_discard_threshold,
                            visible_validity_exponential_decay_factor: &configuration
                                .ball_filter
                                .visible_validity_exponential_decay_factor,
                            balls_bottom: framework::PerceptionInput {
                                persistent: self
                                    .perception_databases
                                    .persistent()
                                    .map(|(system_time, databases)| {
                                        (
                                            *system_time,
                                            databases
                                                .vision_bottom
                                                .iter()
                                                .map(|database| database.balls.as_ref())
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
                                                .vision_bottom
                                                .iter()
                                                .map(|database| database.balls.as_ref())
                                                .collect(),
                                        )
                                    })
                                    .collect(),
                            },
                            balls_top: framework::PerceptionInput {
                                persistent: self
                                    .perception_databases
                                    .persistent()
                                    .map(|(system_time, databases)| {
                                        (
                                            *system_time,
                                            databases
                                                .vision_top
                                                .iter()
                                                .map(|database| database.balls.as_ref())
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
                                                .vision_top
                                                .iter()
                                                .map(|database| database.balls.as_ref())
                                                .collect(),
                                        )
                                    })
                                    .collect(),
                            },
                        })
                        .wrap_err("failed to execute cycle of node `BallFilter`")?;
                    own_database_reference.main_outputs.ball_position =
                        main_outputs.ball_position.value;
                } else {
                    own_database_reference.main_outputs.ball_position = Default::default();
                }
                if own_database_reference
                    .main_outputs
                    .game_controller_state
                    .as_ref()
                    .is_some()
                {
                    let main_outputs = self
                        .game_state_filter
                        .cycle(control::game_state_filter::CycleContext {
                            ball_position: own_database_reference
                                .main_outputs
                                .ball_position
                                .as_ref(),
                            buttons: &own_database_reference.main_outputs.buttons,
                            cycle_time: &own_database_reference.main_outputs.cycle_time,
                            filtered_whistle: &own_database_reference.main_outputs.filtered_whistle,
                            game_controller_state: own_database_reference
                                .main_outputs
                                .game_controller_state
                                .as_ref()
                                .unwrap(),
                            sensor_data: &own_database_reference.main_outputs.sensor_data,
                            config: &configuration.game_state_filter,
                            field_dimensions: &configuration.field_dimensions,
                            player_number: &configuration.player_number,
                            robot_to_field: &mut self.persistent_state.robot_to_field,
                        })
                        .wrap_err("failed to execute cycle of node `GameStateFilter`")?;
                    own_database_reference.main_outputs.filtered_game_state =
                        main_outputs.filtered_game_state.value;
                } else {
                    own_database_reference.main_outputs.filtered_game_state = Default::default();
                }
                {
                    let main_outputs = self
                        .primary_state_filter
                        .cycle(control::primary_state_filter::CycleContext {
                            buttons: &own_database_reference.main_outputs.buttons,
                            filtered_game_state: own_database_reference
                                .main_outputs
                                .filtered_game_state
                                .as_ref(),
                            game_controller_state: own_database_reference
                                .main_outputs
                                .game_controller_state
                                .as_ref(),
                            has_ground_contact: &own_database_reference
                                .main_outputs
                                .has_ground_contact,
                            player_number: &configuration.player_number,
                        })
                        .wrap_err("failed to execute cycle of node `PrimaryStateFilter`")?;
                    own_database_reference.main_outputs.primary_state =
                        main_outputs.primary_state.value;
                }
                if own_database_reference
                    .main_outputs
                    .ball_position
                    .as_ref()
                    .is_some()
                    && own_database_reference
                        .main_outputs
                        .game_controller_state
                        .as_ref()
                        .is_some()
                {
                    let main_outputs = self
                        .penalty_shot_direction_estimation
                        .cycle(control::penalty_shot_direction_estimation::CycleContext {
                            field_dimensions: &configuration.field_dimensions,
                            moving_distance_threshold: &configuration
                                .penalty_shot_direction_estimation
                                .moving_distance_threshold,
                            ball_position: own_database_reference
                                .main_outputs
                                .ball_position
                                .as_ref()
                                .unwrap(),
                            game_controller_state: own_database_reference
                                .main_outputs
                                .game_controller_state
                                .as_ref()
                                .unwrap(),
                            primary_state: &own_database_reference.main_outputs.primary_state,
                        })
                        .wrap_err(
                            "failed to execute cycle of node `PenaltyShotDirectionEstimation`",
                        )?;
                    own_database_reference.main_outputs.penalty_shot_direction =
                        main_outputs.penalty_shot_direction.value;
                } else {
                    own_database_reference.main_outputs.penalty_shot_direction = Default::default();
                }
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
                            sensor_data: &own_database_reference.main_outputs.sensor_data,
                            cycle_time: &own_database_reference.main_outputs.cycle_time,
                            field_dimensions: &configuration.field_dimensions,
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
                                own_subscribed_outputs.iter().any(|subscribed_output| {
                                    framework::should_be_filled(
                                        subscribed_output,
                                        "additional_outputs.kick_decisions",
                                    )
                                }),
                                &mut own_database_reference.additional_outputs.kick_decisions,
                            ),
                            kick_targets: framework::AdditionalOutput::new(
                                own_subscribed_outputs.iter().any(|subscribed_output| {
                                    framework::should_be_filled(
                                        subscribed_output,
                                        "additional_outputs.kick_targets",
                                    )
                                }),
                                &mut own_database_reference.additional_outputs.kick_targets,
                            ),
                            path_obstacles: framework::AdditionalOutput::new(
                                own_subscribed_outputs.iter().any(|subscribed_output| {
                                    framework::should_be_filled(
                                        subscribed_output,
                                        "additional_outputs.path_obstacles",
                                    )
                                }),
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
                {
                    let main_outputs = self
                        .motion_selector
                        .cycle(control::motion::motion_selector::CycleContext {
                            motion_command: &own_database_reference.main_outputs.motion_command,
                            motion_safe_exits: &mut self.persistent_state.motion_safe_exits,
                        })
                        .wrap_err("failed to execute cycle of node `MotionSelector`")?;
                    own_database_reference.main_outputs.motion_selection =
                        main_outputs.motion_selection.value;
                }
                {
                    let main_outputs = self
                        .walk_manager
                        .cycle(control::motion::walk_manager::CycleContext {
                            motion_command: &own_database_reference.main_outputs.motion_command,
                            motion_selection: &own_database_reference.main_outputs.motion_selection,
                            step_plan: &own_database_reference.main_outputs.step_plan,
                        })
                        .wrap_err("failed to execute cycle of node `WalkManager`")?;
                    own_database_reference.main_outputs.walk_command =
                        main_outputs.walk_command.value;
                }
            }
            self.historic_databases.update(
                now,
                self.perception_databases
                    .get_first_timestamp_of_temporary_databases(),
                &own_database_reference.main_outputs,
            );
        }
        self.own_changed.notify_one();
        Ok(())
    }
}
