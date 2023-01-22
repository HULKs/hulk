#[derive(
    Default, serde :: Deserialize, serde :: Serialize, serialize_hierarchy :: SerializeHierarchy,
)]
pub struct Database {
    pub main_outputs: structs::control::MainOutputs,
    pub additional_outputs: structs::control::AdditionalOutputs,
    simulator_database: SimulatorDatabase,
}

#[derive(
    Default, serde :: Deserialize, serde :: Serialize, serialize_hierarchy :: SerializeHierarchy,
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
    sensor_data_receiver: control::sensor_data_receiver::SensorDataReceiver,
    sole_pressure_filter: control::sole_pressure_filter::SolePressureFilter,
    ground_contact_detector: control::ground_contact_detector::GroundContactDetector,
    fall_state_estimation: control::fall_state_estimation::FallStateEstimation,
    sonar_filter: control::sonar_filter::SonarFilter,
    button_filter: control::button_filter::ButtonFilter,
    support_foot_estimation: control::support_foot_estimation::SupportFootEstimation,
    orientation_filter: control::orientation_filter::OrientationFilter,
    kinematics_provider: control::kinematics_provider::KinematicsProvider,
    ground_provider: control::ground_provider::GroundProvider,
    center_of_mass_provider: control::center_of_mass_provider::CenterOfMassProvider,
    whistle_filter: control::whistle_filter::WhistleFilter,
    camera_matrix_calculator: control::camera_matrix_calculator::CameraMatrixCalculator,
    odometry: control::odometry::Odometry,
    limb_projector: control::limb_projector::LimbProjector,
    ball_filter: control::ball_filter::BallFilter,
    game_controller_filter: control::game_controller_filter::GameControllerFilter,
    game_state_filter: control::game_state_filter::GameStateFilter,
    primary_state_filter: control::primary_state_filter::PrimaryStateFilter,
    localization: control::localization::Localization,
    penalty_shot_direction_estimation:
        control::penalty_shot_direction_estimation::PenaltyShotDirectionEstimation,
    role_assignment: control::role_assignment::RoleAssignment,
    obstacle_filter: control::obstacle_filter::ObstacleFilter,
    world_state_composer: control::world_state_composer::WorldStateComposer,
    behavior: control::behavior::node::Behavior,
    step_planner: control::motion::step_planner::StepPlanner,
    look_at: control::motion::look_at::LookAt,
    motion_selector: control::motion::motion_selector::MotionSelector,
    stand_up_back: control::motion::stand_up_back::StandUpBack,
    stand_up_front: control::motion::stand_up_front::StandUpFront,
    walk_manager: control::motion::walk_manager::WalkManager,
    fall_protector: control::motion::fall_protector::FallProtector,
    jump_left: control::motion::jump_left::JumpLeft,
    jump_right: control::motion::jump_right::JumpRight,
    sit_down: control::motion::sit_down::SitDown,
    led_status: control::led_status::LedStatus,
    arms_up_squat: control::motion::arms_up_squat::ArmsUpSquat,
    look_around: control::motion::look_around::LookAround,
    head_motion: control::motion::head_motion::HeadMotion,
    walking_engine: control::motion::walking_engine::WalkingEngine,
    dispatching_interpolator: control::motion::dispatching_interpolator::DispatchingInterpolator,
    joint_command_sender: control::motion::joint_command_sender::JointCommandSender,
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
        todo!()
    }

    pub fn start(
        &self,
        keep_running: tokio_util::sync::CancellationToken,
    ) -> color_eyre::Result<std::thread::JoinHandle<color_eyre::Result<()>>> {
        todo!()
    }
}
