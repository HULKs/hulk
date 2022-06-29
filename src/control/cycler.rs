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

use super::{sensor_data_receiver::receive_sensor_data, Database, PersistentState};

include!(concat!(
    env!("OUT_DIR"),
    "/control_cycler_modules_struct.rs"
));

include!(concat!(
    env!("OUT_DIR"),
    "/control_cycler_modules_initializer.rs"
));

pub struct Control<Hardware>
where
    Hardware: crate::hardware::HardwareInterface + Sync + Send,
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

    modules: ControlModules,
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
        let configuration = communication_channels.configuration.next().clone();
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

            modules: ControlModules::new(&configuration)
                .context("Failed to create control modules")?,
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
            include!(concat!(env!("OUT_DIR"), "/control_cycler_run_cycles.rs"));

            let positions = match control_database.main_outputs.positions {
                Some(joints) => joints,
                None => {
                    error!(
                        "Joint angles were None. MainOutputs: {:#?}",
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
