use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};

use alsa::{
    pcm::{Access, Format, HwParams},
    Direction, ValueOr, PCM,
};
use anyhow::Context;
use parking_lot::Mutex;

use crate::{
    hardware::{
        interface::{AUDIO_SAMPLE_RATE, NUMBER_OF_AUDIO_CHANNELS, NUMBER_OF_AUDIO_SAMPLES},
        HardwareIds, HardwareInterface,
    },
    types::{CameraPosition, CycleInfo, Image422, Joints, Leds, SensorData},
};

use super::{hula_interface::HulaInterface, nao_camera::NaoCamera};

pub struct NaoInterface {
    interface: Mutex<HulaInterface>,
    ids: HardwareIds,
    top_camera: Mutex<NaoCamera>,
    top_image: Mutex<Image422>,
    bottom_camera: Mutex<NaoCamera>,
    bottom_image: Mutex<Image422>,
    audio_device: Mutex<PCM>,
    audio_buffer: Mutex<[[f32; NUMBER_OF_AUDIO_SAMPLES]; NUMBER_OF_AUDIO_CHANNELS]>,
}

impl NaoInterface {
    pub fn new() -> anyhow::Result<Self> {
        let interface = HulaInterface::new()?;
        let ids = interface.get_ids();
        let audio_device = Self::new_audio_device().context("Failed to initialize audio device")?;
        let i2c_head_mutex = Arc::new(Mutex::new(()));

        Ok(Self {
            interface: Mutex::new(interface),
            ids,
            top_camera: Mutex::new(NaoCamera::new(
                "/dev/video-top",
                CameraPosition::Top,
                i2c_head_mutex.clone(),
            )?),
            top_image: Mutex::new(Image422::zero(0, 0)),
            bottom_camera: Mutex::new(NaoCamera::new(
                "/dev/video-bottom",
                CameraPosition::Bottom,
                i2c_head_mutex,
            )?),
            bottom_image: Mutex::new(Image422::zero(0, 0)),
            audio_device: Mutex::new(audio_device),
            audio_buffer: Mutex::new([[0.0; NUMBER_OF_AUDIO_SAMPLES]; NUMBER_OF_AUDIO_CHANNELS]),
        })
    }

    fn new_audio_device() -> anyhow::Result<PCM> {
        let device = PCM::new("default", Direction::Capture, false)
            .context("Failed to open audio device")?;
        {
            let hardware_parameters =
                HwParams::any(&device).context("Failed to create hardware parameters")?;
            hardware_parameters
                .set_access(Access::RWInterleaved)
                .context("Failed to set access")?;
            hardware_parameters
                .set_format(Format::FloatLE)
                .context("Failed to set format")?;
            hardware_parameters
                .set_rate_near(AUDIO_SAMPLE_RATE, ValueOr::Nearest)
                .context("Failed to set sample rate")?;
            hardware_parameters
                .set_channels(NUMBER_OF_AUDIO_CHANNELS as u32)
                .context("Failed to set channel")?;
            device
                .hw_params(&hardware_parameters)
                .context("Failed to set hardware parameters")?;
        }
        device.prepare().context("Failed to prepare device")?;
        Ok(device)
    }
}

impl HardwareInterface for NaoInterface {
    fn get_ids(&self) -> HardwareIds {
        self.ids.clone()
    }

    fn set_leds(&self, leds: Leds) {
        let mut interface = self.interface.lock();
        interface.set_leds(leds);
    }

    fn set_joint_positions(&self, requested_positions: Joints) {
        let mut interface = self.interface.lock();
        interface.set_joint_positions(requested_positions);
    }

    fn set_joint_stiffnesses(&self, requested_stiffnesses: Joints) {
        let mut interface = self.interface.lock();
        interface.set_joint_stiffnesses(requested_stiffnesses);
    }

    fn produce_sensor_data(&self) -> anyhow::Result<SensorData> {
        let mut interface = self.interface.lock();
        interface.produce_sensor_data()
    }

    fn produce_image_data(&self, camera_position: CameraPosition) -> anyhow::Result<CycleInfo> {
        match camera_position {
            CameraPosition::Top => {
                let mut image = self.top_image.lock();
                *image = self.top_camera.lock().get_next_image()?;
            }
            CameraPosition::Bottom => {
                let mut image = self.bottom_image.lock();
                *image = self.bottom_camera.lock().get_next_image()?;
            }
        }
        Ok(CycleInfo {
            start_time: SystemTime::now(),
            last_cycle_duration: Duration::from_millis(30),
        })
    }

    fn get_image(&self, camera_position: CameraPosition) -> &Mutex<Image422> {
        match camera_position {
            CameraPosition::Top => &self.top_image,
            CameraPosition::Bottom => &self.bottom_image,
        }
    }

    fn start_image_capture(&self, camera_position: CameraPosition) -> anyhow::Result<()> {
        match camera_position {
            CameraPosition::Top => self.top_camera.lock().start(),
            CameraPosition::Bottom => self.bottom_camera.lock().start(),
        }
    }

    fn produce_audio_data(&self) -> anyhow::Result<()> {
        let audio_device = self.audio_device.lock();
        let io_device = audio_device
            .io_f32()
            .context("Failed to create I/O device")?;
        let mut interleaved_buffer = [0.0; NUMBER_OF_AUDIO_CHANNELS * NUMBER_OF_AUDIO_SAMPLES];
        let number_of_frames = io_device
            .readi(&mut interleaved_buffer)
            .context("Failed to read audio data")?;
        let mut non_interleaved_buffer = self.audio_buffer.lock();
        for channel_index in 0..NUMBER_OF_AUDIO_CHANNELS {
            for frame_index in 0..number_of_frames {
                non_interleaved_buffer[channel_index][frame_index] =
                    interleaved_buffer[frame_index * NUMBER_OF_AUDIO_CHANNELS + channel_index];
            }
        }
        Ok(())
    }

    fn get_audio_buffer(
        &self,
    ) -> &Mutex<[[f32; NUMBER_OF_AUDIO_SAMPLES]; NUMBER_OF_AUDIO_CHANNELS]> {
        &self.audio_buffer
    }
}
