use libc;
use std::time::Duration;

use anyhow::{anyhow, Context};
use log::{debug, warn};
use polling::{Event, Poller};
use v4l::{
    buffer::Type,
    io::{
        mmap,
        traits::{CaptureStream, Stream},
    },
    video::Capture,
    Device, Format, FourCC, Fraction,
};

use crate::{
    hardware::nao::{
        registers::write_register,
        v4l2::{
            apply_v4l2_settings, ExposureMode, FocusMode, HueMode, V4L2Controls, WhiteBalanceMode,
        },
    },
    types::{CameraPosition, Image422},
};

pub const UVC_EXTENSION_UNIT: u8 = 0x03;

pub struct NaoCamera {
    stream: mmap::Stream<'static>,
    device: Device,
    polling_key: usize,
    poller: Poller,
    last_sequence_number: u32,
    last_image_timestamp: Duration,
}

impl NaoCamera {
    const FOURCC: FourCC = FourCC { repr: *b"YUYV" };
    const BUFFER_COUNT: usize = 4;
    const IMAGE_WIDTH: u32 = 640;
    const IMAGE_HEIGHT: u32 = 480;

    pub fn new(device_path: &str, camera_position: CameraPosition) -> anyhow::Result<Self> {
        let device = Device::with_path_and_flags(device_path, libc::O_NONBLOCK | libc::O_RDWR)
            .with_context(|| format!("Failed to open camera device {}", device_path))?;
        let applied_format =
            Self::configure_device(&device).context("Failed to configure device")?;
        debug!("Applied format to camera:\n{}", applied_format);
        let controls = V4L2Controls {
            exposure_mode: ExposureMode::Auto,
            white_balance_mode: WhiteBalanceMode::Auto,
            brightness: 0,
            contrast: 32,
            gain: 16,
            hue: 0,
            saturation: 64,
            sharpness: 4,
            white_balance_temperature: 2500,
            exposure_absolute: 512,
            hue_mode: HueMode::Auto,
            focus_mode: FocusMode::Manual,
            focus_absolute: 0,
        };
        apply_v4l2_settings(&device, controls)?;
        if let CameraPosition::Top = camera_position {
            flip_sensor(&device)?
        }
        disable_digital_effects(&device)?;
        set_aec_weights(&device, [1; 16])?;
        let stream =
            mmap::Stream::with_buffers(&device, Type::VideoCapture, Self::BUFFER_COUNT as u32)
                .context("Failed to create buffered stream")?;
        let polling_key = 1;
        let poller = Poller::new()?;
        poller.add(device.handle().fd(), Event::readable(polling_key))?;
        Ok(Self {
            stream,
            device,
            polling_key,
            poller,
            last_sequence_number: 0,
            last_image_timestamp: Duration::ZERO,
        })
    }

    fn configure_device(device: &Device) -> anyhow::Result<Format> {
        let format = Format::new(Self::IMAGE_WIDTH, Self::IMAGE_HEIGHT, Self::FOURCC);
        let actual_format = device.set_format(&format)?;
        let mut parameters = device.params()?;
        parameters.interval = Fraction::new(1, 30);
        device.set_params(&parameters)?;
        Ok(actual_format)
    }

    pub fn start(&mut self) -> anyhow::Result<()> {
        self.stream
            .start()
            .context("Failed to start the capture stream")?;
        for i in 0..Self::BUFFER_COUNT {
            self.stream
                .queue(i)
                .with_context(|| format!("Failed to queue buffer {}", i))?;
        }
        Ok(())
    }

    pub fn get_next_image(&mut self) -> anyhow::Result<Image422> {
        let mut events = Vec::new();
        self.poller.wait(&mut events, None)?;
        self.poller
            .modify(self.device.handle().fd(), Event::readable(self.polling_key))?;
        let is_readable = events.iter().fold(true, |acc, event| match event {
            Event {
                key,
                readable: true,
                writable: false,
            } => acc && key == &self.polling_key,
            _ => false,
        });
        if !is_readable || events.is_empty() {
            return Err(anyhow!("Could not poll event"));
        }
        let mut buffer_index = None;
        while let Ok(index) = self.stream.dequeue() {
            if let Some(previous_buffer_index) = buffer_index {
                self.release_image(previous_buffer_index)?;
            };
            buffer_index = Some(index);
        }

        let buffer_index = buffer_index.context("Failed to dequeue the next buffer")?;
        let buffer = self
            .stream
            .get(buffer_index)
            .expect("Previously queued buffer with index {} does not exist");
        let metadata = self
            .stream
            .get_meta(buffer_index)
            .expect("Previously queued metadata does not exist");
        let sequence_number = metadata.sequence;
        let image_timestamp = Duration::from(metadata.timestamp);
        if self.last_sequence_number + 1 != sequence_number {
            warn!(
                "sequence numbers are not consecutive; last: {} this: {} -> skipped {} frames",
                self.last_sequence_number,
                sequence_number,
                sequence_number - self.last_sequence_number
            );
        }
        debug!(
            "Time between frames: {}ms",
            (image_timestamp - self.last_image_timestamp).as_millis()
        );
        self.last_sequence_number = sequence_number;
        self.last_image_timestamp = image_timestamp;
        let image = Image422::from_slice(
            buffer,
            (Self::IMAGE_WIDTH / 2) as usize,
            Self::IMAGE_HEIGHT as usize,
        );
        self.release_image(buffer_index)?;
        Ok(image)
    }

    fn release_image(&mut self, buffer_index: usize) -> std::io::Result<()> {
        self.stream.queue(buffer_index)
    }
}

fn set_aec_weights(device: &Device, weights: [u8; 16]) -> anyhow::Result<()> {
    assert!(weights.iter().all(|x| *x < 0x10));
    debug!("Setting AEC weights to {:?}", weights);
    let fd = device.handle().fd();
    let bytes = [0; 17];
    let mut bytes = uvcvideo::get_control(fd, UVC_EXTENSION_UNIT, 0x09, &bytes)
        .context("Failed to get control for AEC weights")?;
    for i in (0..weights.len()).step_by(2) {
        bytes[9 + i / 2] = (weights[i + 1] << 4) | weights[i];
    }
    uvcvideo::set_control(fd, UVC_EXTENSION_UNIT, 0x09, &bytes)
        .context("Failed to set control for AEC weights")?;
    Ok(())
}

fn disable_digital_effects(device: &Device) -> anyhow::Result<()> {
    debug!("Disabling digital effects");
    write_register(device, 0x5001, 0b00100011)?;
    Ok(())
}

fn flip_sensor(device: &Device) -> anyhow::Result<()> {
    debug!("Setting rotation");
    let horizontal_flip_unit_selector = 0x0c;
    let vertical_flip_unit_selector = 0x0d;
    uvcvideo::set_control(
        device.handle().fd(),
        UVC_EXTENSION_UNIT,
        horizontal_flip_unit_selector,
        &[1, 0],
    )
    .context("Failed to flip horizontally")?;
    uvcvideo::set_control(
        device.handle().fd(),
        UVC_EXTENSION_UNIT,
        vertical_flip_unit_selector,
        &[1, 0],
    )
    .context("Failed to flip vertically")?;
    Ok(())
}
