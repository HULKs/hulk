use std::{
    sync::Arc,
    thread::{Builder, JoinHandle},
};

use anyhow::Context;
use log::error;
use tokio_util::sync::CancellationToken;

use crate::{
    control,
    framework::{
        buffer::{Reader, Writer},
        future_queue::Producer,
        util::collect_changed_parameters,
    },
    hardware::HardwareInterface,
    types::CameraPosition,
    CommunicationChannelsForCyclerWithImage,
};

use super::{database::MainOutputs, image_receiver::receive_image, Database};

include!(concat!(env!("OUT_DIR"), "/vision_cycler_modules_struct.rs"));

include!(concat!(
    env!("OUT_DIR"),
    "/vision_cycler_modules_initializer.rs"
));

#[allow(dead_code)]
pub struct Vision<Hardware>
where
    Hardware: HardwareInterface + Sync + Send,
{
    instance: CameraPosition,
    hardware_interface: Arc<Hardware>,
    control_reader: Reader<control::Database>,
    vision_writer: Writer<Database>,
    vision_producer: Producer<MainOutputs>,
    communication_channels: CommunicationChannelsForCyclerWithImage,

    modules: VisionModules,
}

impl<Hardware> Vision<Hardware>
where
    Hardware: HardwareInterface + Sync + Send + 'static,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instance: CameraPosition,
        hardware_interface: Arc<Hardware>,
        control_reader: Reader<control::Database>,
        vision_writer: Writer<Database>,
        vision_producer: Producer<MainOutputs>,
        communication_channels: CommunicationChannelsForCyclerWithImage,
    ) -> anyhow::Result<Self> {
        let configuration = communication_channels.configuration.next().clone();
        let cycler_configuration = match instance {
            CameraPosition::Top => &configuration.vision_top,
            CameraPosition::Bottom => &configuration.vision_bottom,
        };
        Ok(Self {
            instance,
            hardware_interface,
            control_reader,
            vision_writer,
            vision_producer,
            communication_channels,

            modules: VisionModules::new(&configuration, cycler_configuration)
                .context("Failed to create vision modules")?,
        })
    }

    pub fn start(mut self, keep_running: CancellationToken) -> JoinHandle<()> {
        let name = match self.instance {
            CameraPosition::Top => "vision_top",
            CameraPosition::Bottom => "vision_bottom",
        };
        Builder::new()
            .name(name.to_string())
            .spawn(move || {
                if let Err(error) = self.hardware_interface.start_image_capture(self.instance) {
                    error!("Failed to start capture on hardware interface: {:?}", error);
                }
                while !keep_running.is_cancelled() {
                    if let Err(error) = self.cycle() {
                        error!("`cycle` returned error: {:?}", error);
                        keep_running.cancel();
                    }
                }
            })
            .expect("Failed to spawn thread")
    }

    fn cycle(&mut self) -> anyhow::Result<()> {
        {
            let mut vision_database = self.vision_writer.next();

            // prepare
            vision_database.main_outputs.cycle_info =
                Some(receive_image(&*self.hardware_interface, self.instance)?);

            self.vision_producer.announce();

            let configuration = self.communication_channels.configuration.next();
            let cycler_configuration = match self.instance {
                CameraPosition::Top => &configuration.vision_top,
                CameraPosition::Bottom => &configuration.vision_bottom,
            };
            let control_database = self.control_reader.next();
            let subscribed_additional_outputs = self
                .communication_channels
                .subscribed_additional_outputs
                .next();
            let changed_parameters =
                collect_changed_parameters(&mut self.communication_channels.changed_parameters)?;

            let image = self.hardware_interface.get_image(self.instance).lock();

            if *self.communication_channels.subscribed_image.next() {
                vision_database.image = Some((*image).clone());
            }

            // process
            include!(concat!(env!("OUT_DIR"), "/vision_cycler_run_cycles.rs"));

            self.vision_producer
                .finalize(vision_database.main_outputs.clone());
        }

        self.communication_channels.database_changed.notify_one();

        Ok(())
    }
}
