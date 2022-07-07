use std::{
    sync::Arc,
    thread::{Builder, JoinHandle},
};

use anyhow::Result;
use log::error;
use rustfft::{Fft, FftPlanner};
use tokio_util::sync::CancellationToken;
use types::Whistle;

use crate::{
    control,
    framework::{
        buffer::{Reader, Writer},
        future_queue::Producer,
    },
    hardware::{HardwareInterface, NUMBER_OF_AUDIO_SAMPLES},
    CommunicationChannelsForCycler,
};

use super::{
    database::MainOutputs,
    microphone_recorder::record_microphone,
    whistle_detection::{self, is_whistle_detected_in_buffer},
    Database,
};

#[allow(dead_code)]
pub struct Audio<Hardware>
where
    Hardware: HardwareInterface + Sync + Send,
{
    hardware_interface: Arc<Hardware>,
    control_reader: Reader<control::Database>,
    audio_writer: Writer<Database>,
    audio_producer: Producer<MainOutputs>,
    communication_channels: CommunicationChannelsForCycler,
    fft: Arc<dyn Fft<f32>>,
}

impl<Hardware> Audio<Hardware>
where
    Hardware: HardwareInterface + Sync + Send + 'static,
{
    pub fn new(
        hardware_interface: Arc<Hardware>,
        control_reader: Reader<control::Database>,
        audio_writer: Writer<Database>,
        audio_producer: Producer<MainOutputs>,
        communication_channels: CommunicationChannelsForCycler,
    ) -> anyhow::Result<Self> {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(NUMBER_OF_AUDIO_SAMPLES);
        Ok(Self {
            hardware_interface,
            control_reader,
            audio_writer,
            audio_producer,
            communication_channels,
            fft,
        })
    }

    pub fn start(mut self, keep_running: CancellationToken) -> JoinHandle<()> {
        Builder::new()
            .name("audio".to_string())
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
            let mut audio_database = self.audio_writer.next();

            // prepare
            let buffer = record_microphone(&*self.hardware_interface)?;

            self.audio_producer.announce();

            let configuration = self.communication_channels.configuration.next();
            let subscribed_additional_outputs = self
                .communication_channels
                .subscribed_additional_outputs
                .next();

            // process
            audio_database.main_outputs.detected_whistle = Some(Whistle {
                is_detected: is_whistle_detected_in_buffer(
                    self.fft.clone(),
                    &buffer.lock(),
                    &configuration.audio.whistle_detection.detection_band,
                    configuration
                        .audio
                        .whistle_detection
                        .background_noise_scaling,
                    configuration.audio.whistle_detection.whistle_scaling,
                    configuration.audio.whistle_detection.number_of_chunks,
                    whistle_detection::AdditionalOutputs::new(
                        &mut audio_database.additional_outputs,
                        &subscribed_additional_outputs,
                    ),
                )?,
            });

            self.audio_producer
                .finalize(audio_database.main_outputs.clone());
        }

        self.communication_channels.database_changed.notify_one();

        Ok(())
    }
}
