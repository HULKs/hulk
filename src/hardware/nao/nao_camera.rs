use std::{
    path::{Path, PathBuf},
    sync::Arc,
    thread::sleep,
    time::Duration,
};

use anyhow::{bail, Context};
use i2cdev::{core::I2CDevice, linux::LinuxI2CDevice};
use log::{debug, info, warn};
use parking_lot::Mutex;
use types::{CameraPosition, Image422};
use v4l::{
    buffer::Type,
    device::{OpenFlags, WaitError},
    io::{
        mmap,
        traits::{CaptureStream, Stream},
    },
    video::Capture,
    Device, Format, FourCC, Fraction,
};

use crate::hardware::nao::{
    registers::write_register,
    v4l2::{apply_v4l2_settings, ExposureMode, FocusMode, HueMode, V4L2Controls, WhiteBalanceMode},
};

pub const UVC_EXTENSION_UNIT: u8 = 0x03;

pub struct NaoCamera {
    device: Device,
    stream: mmap::Stream<'static>,
    last_sequence_number: u32,
    last_image_timestamp: Duration,
    camera_position: CameraPosition,
    device_path: PathBuf,
    i2c_head_mutex: Arc<Mutex<()>>,
}

impl NaoCamera {
    const FOURCC: FourCC = FourCC { repr: *b"YUYV" };
    const BUFFER_COUNT: usize = 4;
    const IMAGE_WIDTH: u32 = 640;
    const IMAGE_HEIGHT: u32 = 480;

    pub fn new<P: Into<PathBuf>>(
        device_path: P,
        camera_position: CameraPosition,
        i2c_head_mutex: Arc<Mutex<()>>,
    ) -> anyhow::Result<Self> {
        let device_path: PathBuf = device_path.into();
        Self::reset_camera_device(camera_position, &device_path, &i2c_head_mutex)
            .context("Reset of camera device failed")?;
        let (device, stream) = Self::create_device_and_stream(&device_path, camera_position)
            .context("Creating device and stream failed")?;
        Ok(Self {
            device,
            stream,
            last_sequence_number: 0,
            last_image_timestamp: Duration::ZERO,
            camera_position,
            device_path,
            i2c_head_mutex,
        })
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

    pub fn stop(&mut self) -> anyhow::Result<()> {
        self.stream.stop().context("Stopping stream failed")?;
        self.device.close();
        Ok(())
    }

    pub fn get_next_image(&mut self) -> anyhow::Result<Image422> {
        const MAXIMUM_NUMBER_OF_RETRIES: i32 = 10;
        for i in (0..MAXIMUM_NUMBER_OF_RETRIES).rev() {
            if i <= 0 {
                bail!("Timed out trying to restart camera device");
            }

            const IMAGE_CAPTURE_TIMEOUT: usize = 1000;
            match self.device.wait(Some(IMAGE_CAPTURE_TIMEOUT)) {
                Err(WaitError::Timeout) => {
                    info!(
                        "Reinitializing {:?} camera device. Please wait.",
                        self.camera_position
                    );
                    self.reinitialize_camera()
                        .context("Reinitializing camera failed")?;
                    continue;
                }
                result => result.context("Failed to retrieve an image")?,
            }
            break;
        }
        // if more than 1 image has been captured in the previous cycle,
        // discard all images except for the newest one
        let mut buffer_index = None;
        while let Ok(index) = self.stream.dequeue() {
            if let Some(previous_buffer_index) = buffer_index {
                self.release_image(previous_buffer_index)
                    .context("Failed to release image")?;
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
        self.release_image(buffer_index)
            .context("Failed to release image")?;
        Ok(image)
    }

    fn configure_device(device: &Device) -> anyhow::Result<Format> {
        let format = Format::new(Self::IMAGE_WIDTH, Self::IMAGE_HEIGHT, Self::FOURCC);
        let format = device.set_format(&format).context("Failed to set format")?;
        let mut parameters = device.params().context("Failed to get parameters")?;
        parameters.interval = Fraction::new(1, 30);
        device
            .set_params(&parameters)
            .context("Failed to set parameters")?;
        Ok(format)
    }

    fn reinitialize_camera(&mut self) -> anyhow::Result<()> {
        self.stop().context("Failed to stop camera device")?;
        Self::reset_camera_device(
            self.camera_position,
            &self.device_path,
            &self.i2c_head_mutex,
        )
        .context("Failed to reset camera device")?;
        (self.device, self.stream) =
            Self::create_device_and_stream(&self.device_path, self.camera_position)
                .context("Failed to create device and stream")?;
        self.start().context("Failed to start camera device")?;
        Ok(())
    }

    fn create_device_and_stream(
        device_path: &Path,
        camera_position: CameraPosition,
    ) -> anyhow::Result<(Device, mmap::Stream<'static>)> {
        let device = Device::with_path_and_flags(device_path, OpenFlags::Nonblocking)
            .with_context(|| format!("Failed to open camera device {}", device_path.display()))?;

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
        apply_v4l2_settings(&device, controls).context("Failed to apply v4l2 settings")?;
        if let CameraPosition::Top = camera_position {
            flip_sensor(&device).context("Failed to flip sensor")?
        }
        disable_digital_effects(&device).context("Failed to disable digital effects")?;
        set_aec_weights(&device, [1; 16]).context("Failed to set aec weights")?;
        let stream =
            mmap::Stream::with_buffers(&device, Type::VideoCapture, Self::BUFFER_COUNT as u32)
                .context("Failed to create buffered stream")?;
        Ok((device, stream))
    }

    fn reset_camera_device(
        camera_position: CameraPosition,
        device_path: &Path,
        i2c_head_mutex: &Mutex<()>,
    ) -> anyhow::Result<()> {
        let _lock = i2c_head_mutex.lock();
        const SLAVE_ADDRESS: u16 = 0x41;
        let mut device = LinuxI2CDevice::new("/dev/i2c-head", SLAVE_ADDRESS)
            .context("Failed to create new LinuxI2CDevice")?;
        // make sure reset lines of the cameras are configured as outputs on GPIO chip
        if device
            .smbus_read_byte_data(0x3)
            .context("Failed to read i2c data")?
            & 0xc
            != 0x0
        {
            device
                .smbus_write_byte_data(0x1, 0x0)
                .context("Failed to write i2c data")?;
            device
                .smbus_write_byte_data(0x3, 0xf3)
                .context("Failed to write i2c data")?;
        }
        // GPIO pin layout:
        // - pin 0 (usb hub reset): input
        // - pin 1 (reserved):      input
        // - pin 2 (CX3 top camera reset): output (high -> running)
        // - pin 3 (CX3 bottom camera reset): output (high -> running)
        let i2c_set_value = match camera_position {
            CameraPosition::Top => 0x8,
            CameraPosition::Bottom => 0x4,
        };
        // disconnect this camera
        device
            .smbus_write_byte_data(0x1, i2c_set_value)
            .context("Failed to write i2c data")?;
        // wait until USB disconnect has been registered by linux
        let path_buffer = PathBuf::from(device_path);
        let error_message = "Timeout while waiting for camera to disconnect during reset";
        let sleep_time = Duration::from_millis(50);
        poll_condition(&|| !path_buffer.exists(), 20, sleep_time, error_message)?;
        // connect both cameras (if not already connected)
        device
            .smbus_write_byte_data(0x1, 0xc)
            .context("Failed to write i2c data")?;
        // wait until USB disconnect has been registered by linux
        let error_message = "Timeout while waiting for camera to connect during reset";
        poll_condition(&|| path_buffer.exists(), 40, sleep_time, error_message)?;
        Ok(())
    }

    fn release_image(&mut self, buffer_index: usize) -> std::io::Result<()> {
        self.stream.queue(buffer_index)
    }
}

fn poll_condition<F>(
    condition: F,
    maximum_iteration_count: u32,
    sleep_time: Duration,
    error_message: &'static str,
) -> anyhow::Result<()>
where
    F: Fn() -> bool,
{
    for _ in 0..maximum_iteration_count {
        if condition() {
            return Ok(());
        }
        sleep(sleep_time);
    }
    bail!(error_message);
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
    write_register(device, 0x5001, 0b00100011).context("Failed to write register")?;
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
