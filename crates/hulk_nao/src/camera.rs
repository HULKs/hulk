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
use types::{
    camera_position::CameraPosition,
    camera_result::{CameraResult, SequenceNumber},
    ycbcr422_image::YCbCr422Image,
};

pub struct Camera {
    camera: RwLock<Option<NaoCamera>>,
    path: PathBuf,
    camera_position: CameraPosition,
    read_mutex: Mutex<()>,
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
            camera: RwLock::new(None),
            path: path.as_ref().to_path_buf(),
            camera_position,
            read_mutex: Mutex::new(()),
            parameters,
            i2c_head_mutex,
            last_image: Arc::new(RwLock::new(None)),
        };
        camera.reset().wrap_err("failed to reset")?;
        Ok(camera)
    }

    fn has_new_image_for_client(&self, client_sequence_number: &SequenceNumber) -> Result<bool> {
        Ok(self
            .last_image
            .read()
            .as_ref()
            .is_some_and(|last_image| last_image.sequence_number > *client_sequence_number))
    }

    pub fn read(&self, client_sequence_number: &SequenceNumber) -> Result<CameraResult> {
        if self.has_new_image_for_client(client_sequence_number)? {
            return Ok(self.last_image.read().clone().unwrap());
        }

        let _lock = self.read_mutex.lock();
        // only one client is allowed to read the camera
        if self.has_new_image_for_client(client_sequence_number)? {
            return Ok(self.last_image.read().clone().unwrap());
        }

        self.wait_for_device()
            .wrap_err("failed to wait for device")?;

        let mut camera_lock = self.camera.write();
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
        let new_image = YCbCr422Image::from_raw_buffer(
            self.parameters.width / 2,
            self.parameters.height,
            buffer,
        );

        {
            let mut last_image = self.last_image.write();
            *last_image = Some(CameraResult {
                sequence_number: last_image
                    .as_ref()
                    .map(|last_image| last_image.sequence_number.next())
                    .unwrap_or(SequenceNumber::new(1)),
                image: new_image,
            });
        }

        return Ok(self.last_image.read().clone().unwrap());

        // TODO: readd consecutive sequence number checking
    }

    fn wait_for_device(&self) -> Result<()> {
        const MAXIMUM_NUMBER_OF_RETRIES: i32 = 10;
        for _ in 0..MAXIMUM_NUMBER_OF_RETRIES {
            const IMAGE_CAPTURE_TIMEOUT: Duration = Duration::from_secs(1);
            match self
                .camera
                .read()
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
        *self.camera.write() = Some(camera);
        Ok(())
    }
}
