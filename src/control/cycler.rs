use std::{
    sync::Arc,
    thread::{Builder, JoinHandle},
};

use anyhow::{Context, Result};
use log::error;
use tokio_util::sync::CancellationToken;

use crate::{
    audio,
    framework::{
        buffer::Writer, future_queue::Consumer, util::collect_changed_parameters,
        HistoricDatabases, PerceptionDatabases,
    },
    hardware::HardwareInterface,
    spl_network, vision, CommunicationChannelsForCycler,
};

use super::{
    database::PersistentState,
    modules::{
        BallFilter, Behavior, ButtonFilter, CameraMatrixProvider, CenterOfMassProvider,
        DispatchingBodyInterpolator, DispatchingHeadInterpolator, FallProtection,
        FallStateEstimation, GameControllerFilter, GameStateFilter, GroundContactDetector,
        GroundProvider, JointCommandSender, KinematicsProvider, LedStatus, LookAround, LookAt,
        MotionSelector, Odometry, OrientationFilter, PathPlanner, PoseEstimation,
        PrimaryStateFilter, SitDown, SolePressureFilter, StandUpBack, StandUpFront, StepPlanner,
        SupportFootEstimation, WalkManager, WalkingEngine, WhistleFilter, WorldStateComposer,
        ZeroAnglesHead,
    },
    sensor_data_receiver::receive_sensor_data,
    Database,
};

pub struct Control<Hardware>
where
    Hardware: HardwareInterface + Sync + Send,
{
    hardware_interface: Arc<Hardware>,
    control_writer: Writer<Database>,
    spl_network_consumer: Consumer<spl_network::MainOutputs>,
    vision_top_consumer: Consumer<vision::MainOutputs>,
    vision_bottom_consumer: Consumer<vision::MainOutputs>,
    audio_consumer: Consumer<audio::MainOutputs>,
    communication_channels: CommunicationChannelsForCycler,

    historic_databases: HistoricDatabases,
    perception_databases: PerceptionDatabases,

    persistent_state: PersistentState,

    ball_filter: BallFilter,
    behavior: Behavior,
    button_filter: ButtonFilter,
    camera_matrix_provider: CameraMatrixProvider,
    center_of_mass_provider: CenterOfMassProvider,
    dispatching_body_interpolator: DispatchingBodyInterpolator,
    dispatching_head_interpolator: DispatchingHeadInterpolator,
    fall_state_estimation: FallStateEstimation,
    game_controller_filter: GameControllerFilter,
    game_state_filter: GameStateFilter,
    ground_contact_detector: GroundContactDetector,
    ground_provider: GroundProvider,
    joint_command_sender: JointCommandSender,
    kinematics_provider: KinematicsProvider,
    led_status: LedStatus,
    look_around: LookAround,
    look_at: LookAt,
    motion_dispatcher: MotionSelector,
    odometry: Odometry,
    orientation_filter: OrientationFilter,
    path_planner: PathPlanner,
    pose_estimation: PoseEstimation,
    primary_state_filter: PrimaryStateFilter,
    sit_down: SitDown,
    sole_pressure_filter: SolePressureFilter,
    stand_up_back: StandUpBack,
    stand_up_front: StandUpFront,
    step_planner: StepPlanner,
    support_foot_estimation: SupportFootEstimation,
    walk_manager: WalkManager,
    walking_engine: WalkingEngine,
    whistle_filter: WhistleFilter,
    world_state_composer: WorldStateComposer,
    zero_angles_head: ZeroAnglesHead,
    fall_protection: FallProtection,
}

impl<Hardware> Control<Hardware>
where
    Hardware: HardwareInterface + Sync + Send + 'static,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        hardware_interface: Arc<Hardware>,
        control_writer: Writer<Database>,
        spl_network_consumer: Consumer<spl_network::MainOutputs>,
        vision_top_consumer: Consumer<vision::MainOutputs>,
        vision_bottom_consumer: Consumer<vision::MainOutputs>,
        audio_consumer: Consumer<audio::MainOutputs>,
        communication_channels: CommunicationChannelsForCycler,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            hardware_interface,
            control_writer,
            spl_network_consumer,
            vision_top_consumer,
            vision_bottom_consumer,
            audio_consumer,
            communication_channels,

            historic_databases: Default::default(),
            perception_databases: Default::default(),

            persistent_state: Default::default(),

            ball_filter: BallFilter::new(),
            behavior: Behavior::new(),
            button_filter: ButtonFilter::new(),
            camera_matrix_provider: CameraMatrixProvider::new(),
            center_of_mass_provider: CenterOfMassProvider::new(),
            dispatching_body_interpolator: DispatchingBodyInterpolator::new(),
            dispatching_head_interpolator: DispatchingHeadInterpolator::new(),
            fall_state_estimation: FallStateEstimation::new(),
            game_controller_filter: GameControllerFilter::new(),
            game_state_filter: GameStateFilter::new(),
            ground_contact_detector: GroundContactDetector::new(),
            ground_provider: GroundProvider::new(),
            joint_command_sender: JointCommandSender::new(),
            kinematics_provider: KinematicsProvider::new(),
            led_status: LedStatus::new(),
            look_at: LookAt::new(),
            look_around: LookAround::new(),
            motion_dispatcher: MotionSelector::new(),
            odometry: Odometry::new(),
            orientation_filter: OrientationFilter::new(),
            path_planner: PathPlanner::new(),
            pose_estimation: PoseEstimation::new(),
            primary_state_filter: PrimaryStateFilter::new(),
            sit_down: SitDown::new()?,
            sole_pressure_filter: SolePressureFilter::new(),
            stand_up_back: StandUpBack::new()?,
            stand_up_front: StandUpFront::new()?,
            step_planner: StepPlanner::new(),
            support_foot_estimation: SupportFootEstimation::new(),
            walk_manager: WalkManager::new(),
            walking_engine: WalkingEngine::new(),
            whistle_filter: WhistleFilter::new(),
            world_state_composer: WorldStateComposer::new(),
            zero_angles_head: ZeroAnglesHead::new(),
            fall_protection: FallProtection::new(),
        })
    }

    pub fn start(mut self, keep_running: CancellationToken) -> JoinHandle<()> {
        Builder::new()
            .name("control".to_string())
            .spawn(move || {
                while !keep_running.is_cancelled() {
                    if let Err(error) = self.cycle() {
                        error!("`cycle` returned error: {:?}", error);
                        keep_running.cancel();
                    }
                }
            })
            .expect("Failed to spawn thread")
    }

    fn cycle(&mut self) -> Result<()> {
        {
            let mut control_database = self.control_writer.next();

            // prepare
            let main_outputs = receive_sensor_data(&*self.hardware_interface)
                .context("Failed to receive sensor data")?;

            control_database.main_outputs.sensor_data = Some(main_outputs);

            let cycle_start_time = control_database
                .main_outputs
                .sensor_data
                .as_ref()
                .unwrap()
                .cycle_info
                .start_time;
            let audio_update = self.audio_consumer.consume(cycle_start_time);
            let spl_network_update = self.spl_network_consumer.consume(cycle_start_time);
            let vision_top_update = self.vision_top_consumer.consume(cycle_start_time);
            let vision_bottom_update = self.vision_bottom_consumer.consume(cycle_start_time);

            self.perception_databases.update(
                cycle_start_time,
                audio_update,
                spl_network_update,
                vision_top_update,
                vision_bottom_update,
            );

            let configuration = self.communication_channels.configuration.next();

            let subscribed_additional_outputs = self
                .communication_channels
                .subscribed_additional_outputs
                .next();

            let changed_parameters =
                collect_changed_parameters(&mut self.communication_channels.changed_parameters)?;

            // process
            self.kinematics_provider.run_cycle(
                cycle_start_time,
                &mut control_database,
                &self.historic_databases,
                &self.perception_databases,
                &configuration,
                &subscribed_additional_outputs,
                &changed_parameters,
                &mut self.persistent_state,
            )?;

            self.center_of_mass_provider.run_cycle(
                cycle_start_time,
                &mut control_database,
                &self.historic_databases,
                &self.perception_databases,
                &configuration,
                &subscribed_additional_outputs,
                &changed_parameters,
                &mut self.persistent_state,
            )?;

            self.sole_pressure_filter.run_cycle(
                cycle_start_time,
                &mut control_database,
                &self.historic_databases,
                &self.perception_databases,
                &configuration,
                &subscribed_additional_outputs,
                &changed_parameters,
                &mut self.persistent_state,
            )?;

            self.ground_contact_detector.run_cycle(
                cycle_start_time,
                &mut control_database,
                &self.historic_databases,
                &self.perception_databases,
                &configuration,
                &subscribed_additional_outputs,
                &changed_parameters,
                &mut self.persistent_state,
            )?;

            self.support_foot_estimation.run_cycle(
                cycle_start_time,
                &mut control_database,
                &self.historic_databases,
                &self.perception_databases,
                &configuration,
                &subscribed_additional_outputs,
                &changed_parameters,
                &mut self.persistent_state,
            )?;

            self.ground_provider.run_cycle(
                cycle_start_time,
                &mut control_database,
                &self.historic_databases,
                &self.perception_databases,
                &configuration,
                &subscribed_additional_outputs,
                &changed_parameters,
                &mut self.persistent_state,
            )?;

            self.camera_matrix_provider.run_cycle(
                cycle_start_time,
                &mut control_database,
                &self.historic_databases,
                &self.perception_databases,
                &configuration,
                &subscribed_additional_outputs,
                &changed_parameters,
                &mut self.persistent_state,
            )?;

            self.orientation_filter.run_cycle(
                cycle_start_time,
                &mut control_database,
                &self.historic_databases,
                &self.perception_databases,
                &configuration,
                &subscribed_additional_outputs,
                &changed_parameters,
                &mut self.persistent_state,
            )?;

            self.odometry.run_cycle(
                cycle_start_time,
                &mut control_database,
                &self.historic_databases,
                &self.perception_databases,
                &configuration,
                &subscribed_additional_outputs,
                &changed_parameters,
                &mut self.persistent_state,
            )?;

            self.button_filter.run_cycle(
                cycle_start_time,
                &mut control_database,
                &self.historic_databases,
                &self.perception_databases,
                &configuration,
                &subscribed_additional_outputs,
                &changed_parameters,
                &mut self.persistent_state,
            )?;

            self.whistle_filter.run_cycle(
                cycle_start_time,
                &mut control_database,
                &self.historic_databases,
                &self.perception_databases,
                &configuration,
                &subscribed_additional_outputs,
                &changed_parameters,
                &mut self.persistent_state,
            )?;

            self.game_controller_filter.run_cycle(
                cycle_start_time,
                &mut control_database,
                &self.historic_databases,
                &self.perception_databases,
                &configuration,
                &subscribed_additional_outputs,
                &changed_parameters,
                &mut self.persistent_state,
            )?;

            self.game_state_filter.run_cycle(
                cycle_start_time,
                &mut control_database,
                &self.historic_databases,
                &self.perception_databases,
                &configuration,
                &subscribed_additional_outputs,
                &changed_parameters,
                &mut self.persistent_state,
            )?;

            self.primary_state_filter.run_cycle(
                cycle_start_time,
                &mut control_database,
                &self.historic_databases,
                &self.perception_databases,
                &configuration,
                &subscribed_additional_outputs,
                &changed_parameters,
                &mut self.persistent_state,
            )?;

            self.fall_state_estimation.run_cycle(
                cycle_start_time,
                &mut control_database,
                &self.historic_databases,
                &self.perception_databases,
                &configuration,
                &subscribed_additional_outputs,
                &changed_parameters,
                &mut self.persistent_state,
            )?;

            self.pose_estimation.run_cycle(
                cycle_start_time,
                &mut control_database,
                &self.historic_databases,
                &self.perception_databases,
                &configuration,
                &subscribed_additional_outputs,
                &changed_parameters,
                &mut self.persistent_state,
            )?;

            self.ball_filter.run_cycle(
                cycle_start_time,
                &mut control_database,
                &self.historic_databases,
                &self.perception_databases,
                &configuration,
                &subscribed_additional_outputs,
                &changed_parameters,
                &mut self.persistent_state,
            )?;

            self.world_state_composer.run_cycle(
                cycle_start_time,
                &mut control_database,
                &self.historic_databases,
                &self.perception_databases,
                &configuration,
                &subscribed_additional_outputs,
                &changed_parameters,
                &mut self.persistent_state,
            )?;

            self.behavior.run_cycle(
                cycle_start_time,
                &mut control_database,
                &self.historic_databases,
                &self.perception_databases,
                &configuration,
                &subscribed_additional_outputs,
                &changed_parameters,
                &mut self.persistent_state,
            )?;

            self.path_planner.run_cycle(
                cycle_start_time,
                &mut control_database,
                &self.historic_databases,
                &self.perception_databases,
                &configuration,
                &subscribed_additional_outputs,
                &changed_parameters,
                &mut self.persistent_state,
            )?;

            self.motion_dispatcher.run_cycle(
                cycle_start_time,
                &mut control_database,
                &self.historic_databases,
                &self.perception_databases,
                &configuration,
                &subscribed_additional_outputs,
                &changed_parameters,
                &mut self.persistent_state,
            )?;

            self.stand_up_back.run_cycle(
                cycle_start_time,
                &mut control_database,
                &self.historic_databases,
                &self.perception_databases,
                &configuration,
                &subscribed_additional_outputs,
                &changed_parameters,
                &mut self.persistent_state,
            )?;

            self.stand_up_front.run_cycle(
                cycle_start_time,
                &mut control_database,
                &self.historic_databases,
                &self.perception_databases,
                &configuration,
                &subscribed_additional_outputs,
                &changed_parameters,
                &mut self.persistent_state,
            )?;

            self.sit_down.run_cycle(
                cycle_start_time,
                &mut control_database,
                &self.historic_databases,
                &self.perception_databases,
                &configuration,
                &subscribed_additional_outputs,
                &changed_parameters,
                &mut self.persistent_state,
            )?;

            self.step_planner.run_cycle(
                cycle_start_time,
                &mut control_database,
                &self.historic_databases,
                &self.perception_databases,
                &configuration,
                &subscribed_additional_outputs,
                &changed_parameters,
                &mut self.persistent_state,
            )?;

            self.walk_manager.run_cycle(
                cycle_start_time,
                &mut control_database,
                &self.historic_databases,
                &self.perception_databases,
                &configuration,
                &subscribed_additional_outputs,
                &changed_parameters,
                &mut self.persistent_state,
            )?;

            self.walking_engine.run_cycle(
                cycle_start_time,
                &mut control_database,
                &self.historic_databases,
                &self.perception_databases,
                &configuration,
                &subscribed_additional_outputs,
                &changed_parameters,
                &mut self.persistent_state,
            )?;

            self.zero_angles_head.run_cycle(
                cycle_start_time,
                &mut control_database,
                &self.historic_databases,
                &self.perception_databases,
                &configuration,
                &subscribed_additional_outputs,
                &changed_parameters,
                &mut self.persistent_state,
            )?;

            self.look_at.run_cycle(
                cycle_start_time,
                &mut control_database,
                &self.historic_databases,
                &self.perception_databases,
                &configuration,
                &subscribed_additional_outputs,
                &changed_parameters,
                &mut self.persistent_state,
            )?;

            self.look_around.run_cycle(
                cycle_start_time,
                &mut control_database,
                &self.historic_databases,
                &self.perception_databases,
                &configuration,
                &subscribed_additional_outputs,
                &changed_parameters,
                &mut self.persistent_state,
            )?;

            self.fall_protection.run_cycle(
                cycle_start_time,
                &mut control_database,
                &self.historic_databases,
                &self.perception_databases,
                &configuration,
                &subscribed_additional_outputs,
                &changed_parameters,
                &mut self.persistent_state,
            )?;

            self.dispatching_body_interpolator.run_cycle(
                cycle_start_time,
                &mut control_database,
                &self.historic_databases,
                &self.perception_databases,
                &configuration,
                &subscribed_additional_outputs,
                &changed_parameters,
                &mut self.persistent_state,
            )?;

            self.dispatching_head_interpolator.run_cycle(
                cycle_start_time,
                &mut control_database,
                &self.historic_databases,
                &self.perception_databases,
                &configuration,
                &subscribed_additional_outputs,
                &changed_parameters,
                &mut self.persistent_state,
            )?;

            self.joint_command_sender.run_cycle(
                cycle_start_time,
                &mut control_database,
                &self.historic_databases,
                &self.perception_databases,
                &configuration,
                &subscribed_additional_outputs,
                &changed_parameters,
                &mut self.persistent_state,
            )?;

            self.led_status.run_cycle(
                cycle_start_time,
                &mut control_database,
                &self.historic_databases,
                &self.perception_databases,
                &configuration,
                &subscribed_additional_outputs,
                &changed_parameters,
                &mut self.persistent_state,
            )?;

            let positions = match control_database.main_outputs.positions {
                Some(joints) => joints,
                None => {
                    error!(
                        "Joint angles were None. MainOutputs: {:?}",
                        control_database.main_outputs
                    );
                    panic!()
                }
            };

            let leds = control_database.main_outputs.leds.to_owned().unwrap();

            self.hardware_interface.set_joint_positions(positions);
            self.hardware_interface
                .set_joint_stiffnesses(control_database.main_outputs.stiffnesses.unwrap());
            self.hardware_interface.set_leds(leds);

            self.historic_databases.update(
                cycle_start_time,
                self.perception_databases
                    .get_first_timestamp_of_temporary_databases(),
                &control_database,
            );
        }

        self.communication_channels.database_changed.notify_one();

        Ok(())
    }
}
