use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use color_eyre::{
    eyre::{bail, Context},
    Result,
};
use nao_camera::{reset_camera_device, Camera as NaoCamera, Parameters, PollingError};
use parking_lot::{Mutex, RwLock};
use types::{camera_position::CameraPosition, ycbcr422_image::YCbCr422Image, camera_result::{CameraResult, SequenceNumber}};

pub struct Camera {
    camera: Mutex<Option<NaoCamera>>,
    path: PathBuf,
    camera_position: CameraPosition,
    parameters: Parameters,
    i2c_head_mutex: Arc<Mutex<()>>,
    last_image: Arc<RwLock<Option<CameraResult>>>,
}

impl Camera {
    pub fn new(
        path: impl AsRef<Path>,
        camera_position: CameraPosition,
        parameters: Parameters,
        i2c_head_mutex: Arc<Mutex<()>>,
    ) -> Result<Self> {
        let camera = Self {
            camera: Mutex::new(None),
            path: path.as_ref().to_path_buf(),
            camera_position,
            parameters,
            i2c_head_mutex,
            last_image: Arc::new(RwLock::new(None)),
        };
        camera.reset().wrap_err("failed to reset")?;
        Ok(camera)
    }

    fn is_client_outdated(&self, client_sequence_number: &SequenceNumber) -> Result<bool> {
        Ok(self
            .last_image
            .read()
            .as_ref()
            .map(|last_image| last_image.sequence_number > *client_sequence_number)
            .unwrap_or(false))
    }

    pub fn read(&self, client_sequence_number: &SequenceNumber) -> Result<CameraResult> {
        if self.is_client_outdated(client_sequence_number)? {
            return Ok(self.last_image.read().clone().unwrap());
        }

        if self.is_client_outdated(client_sequence_number)? {
            return Ok(self.last_image.read().clone().unwrap());
        }

        self.wait_for_device()
            .wrap_err("failed to wait for device")?;
        let buffer = {
            let mut camera_lock = self.camera.lock();
            let camera = camera_lock.as_mut().unwrap();
            let buffer = camera.dequeue().wrap_err("failed to dequeue buffer")?;
            camera
                .queue(vec![
                    0;
                    match self.parameters.format {
                        nao_camera::Format::YUVU =>
                            (4 * self.parameters.width / 2 * self.parameters.height) as usize,
                    }
                ])
                .wrap_err("failed to queue buffer")?;
            buffer
        };

        let new_image = YCbCr422Image::from_raw_buffer(
            self.parameters.width / 2,
            self.parameters.height,
            buffer,
        );
        self.last_image.write().as_mut().map(|last_image| {
            *last_image = CameraResult {
                sequence_number: last_image.sequence_number.next(),
                image: new_image,
            }
        });
        return Ok(self.last_image.read().clone().unwrap());

        // TODO: readd consecutive sequence number checking
    }

    fn wait_for_device(&self) -> Result<()> {
        const MAXIMUM_NUMBER_OF_RETRIES: i32 = 10;
        for _ in 0..MAXIMUM_NUMBER_OF_RETRIES {
            const IMAGE_CAPTURE_TIMEOUT: Duration = Duration::from_secs(1);
            match self
                .camera
                .lock()
                .as_ref()
                .unwrap()
                .poll(Some(IMAGE_CAPTURE_TIMEOUT))
            {
                Ok(_) => {}
                Err(PollingError::DevicePollingTimedOut) => {
                    self.reset().wrap_err("failed to reset")?;
                    continue;
                }
                error => error.wrap_err("failed to poll")?,
            }
            return Ok(());
        }
        bail!("too many unsuccessful waiting retries");
    }

    fn reset(&self) -> Result<()> {
        let _lock = self.i2c_head_mutex.lock();
        self.camera.lock().take();
        reset_camera_device(&self.path, self.camera_position)
            .wrap_err("failed to reset camera device")?;
        let mut camera =
            NaoCamera::open(&self.path, &self.parameters).wrap_err("failed to open")?;
        camera.start().wrap_err("failed to start")?;
        for _ in 0..self.parameters.amount_of_buffers {
            camera
                .queue(vec![
                    0;
                    match self.parameters.format {
                        nao_camera::Format::YUVU =>
                            ((4 * self.parameters.width * self.parameters.height) / 2) as usize,
                    }
                ])
                .wrap_err("failed to queue buffer")?;
        }
        *self.camera.lock() = Some(camera);
        Ok(())
    }
}
