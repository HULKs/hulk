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
use parking_lot::Mutex;
use types::{nao_image::NaoImage, CameraPosition};

pub struct Camera {
    camera: Option<NaoCamera>,
    path: PathBuf,
    camera_position: CameraPosition,
    parameters: Parameters,
    i2c_head_mutex: Arc<Mutex<()>>,
}

impl Camera {
    pub fn new(
        path: impl AsRef<Path>,
        camera_position: CameraPosition,
        parameters: Parameters,
        i2c_head_mutex: Arc<Mutex<()>>,
    ) -> Result<Self> {
        let mut camera = Self {
            camera: None,
            path: path.as_ref().to_path_buf(),
            camera_position,
            parameters,
            i2c_head_mutex,
        };
        camera.reset().wrap_err("failed to reset")?;
        Ok(camera)
    }

    pub fn read(&mut self) -> Result<NaoImage> {
        self.wait_for_device()
            .wrap_err("failed to wait for device")?;
        let camera = self.camera.as_mut().unwrap();
        let buffer = camera.dequeue().wrap_err("failed to dequeue buffer")?;
        camera
            .queue(vec![
                0;
                match self.parameters.format {
                    nao_camera::Format::YUVU =>
                        (4 * self.parameters.width * self.parameters.height) as usize,
                }
            ])
            .wrap_err("failed to queue buffer")?;
        Ok(NaoImage::from_raw_buffer(
            self.parameters.width / 2,
            self.parameters.height,
            buffer,
        ))
        // TODO: readd consecutive sequence number checking
    }

    fn wait_for_device(&mut self) -> Result<()> {
        const MAXIMUM_NUMBER_OF_RETRIES: i32 = 10;
        for _ in 0..MAXIMUM_NUMBER_OF_RETRIES {
            const IMAGE_CAPTURE_TIMEOUT: Duration = Duration::from_secs(1);
            match self
                .camera
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

    fn reset(&mut self) -> Result<()> {
        let _lock = self.i2c_head_mutex.lock();
        self.camera.take();
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
                            (4 * self.parameters.width * self.parameters.height) as usize,
                    }
                ])
                .wrap_err("failed to queue buffer")?;
        }
        self.camera = Some(camera);
        Ok(())
    }
}
