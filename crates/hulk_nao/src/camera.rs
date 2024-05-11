use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use color_eyre::{
    eyre::{bail, eyre, Context},
    Result,
};
use nao_camera::{reset_camera_device, Camera as NaoCamera, Parameters, PollingError};
use parking_lot::Mutex;
use types::{camera_position::CameraPosition, ycbcr422_image::YCbCr422Image};
use watch::WatchSender as Sender;

pub struct Camera {
    camera: Mutex<CameraHardware>,
    image_sender: Sender<Option<YCbCr422Image>>,
}
pub struct CameraHardware {
    i2c_head_mutex: Arc<Mutex<()>>,
    camera: Option<NaoCamera>,
    config: CameraConfiguration,
    camera_position: CameraPosition,
}

pub struct CameraConfiguration {
    path: PathBuf,
    parameters: Parameters,
}

impl Camera {
    pub fn new(
        path: impl AsRef<Path>,
        camera_position: CameraPosition,
        parameters: Parameters,
        i2c_head_mutex: Arc<Mutex<()>>,
    ) -> Result<Self> {
        let (sender, _) = watch::channel(None);
        let camera = Self {
            camera: Mutex::new(CameraHardware {
                i2c_head_mutex,
                config: CameraConfiguration {
                    path: path.as_ref().to_path_buf(),
                    parameters,
                },
                camera: None,
                camera_position,
            }),
            image_sender: sender,
        };

        camera.camera.lock().reset().wrap_err("failed to reset")?;

        Ok(camera)
    }

    pub fn read(&self) -> Result<YCbCr422Image> {
        let mut this_receiver = self.image_sender.subscribe();

        if let Some(mut camera) = self.camera.try_lock() {
            let new_image = camera.get_next_image()?;
            self.image_sender.send(Some(new_image.clone()));
            return Ok(new_image);
        }

        let image = this_receiver.wait();
        Ok(image.unwrap())
        // TODO: read consecutive sequence number checking
    }
}

impl CameraHardware {
    fn get_next_image(&mut self) -> Result<YCbCr422Image> {
        self.wait_for_device()
            .wrap_err("failed to wait for device")?;

        let camera = self
            .camera
            .as_mut()
            .ok_or_else(|| eyre!("camera does not exist"))?;
        let buffer = camera.dequeue().wrap_err("failed to dequeue buffer")?;
        camera
            .queue(vec![
                0;
                match self.config.parameters.format {
                    nao_camera::Format::YUVU =>
                        (4 * self.config.parameters.width / 2 * self.config.parameters.height)
                            as usize,
                }
            ])
            .wrap_err("failed to queue buffer")?;

        Ok(YCbCr422Image::from_raw_buffer(
            self.config.parameters.width / 2,
            self.config.parameters.height,
            buffer,
        ))
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

        reset_camera_device(&self.config.path, self.camera_position)
            .wrap_err("failed to reset camera device")?;
        let mut camera = NaoCamera::open(&self.config.path, &self.config.parameters)
            .wrap_err("failed to open")?;
        camera.start().wrap_err("failed to start")?;
        for _ in 0..self.config.parameters.amount_of_buffers {
            camera
                .queue(vec![
                    0;
                    match self.config.parameters.format {
                        nao_camera::Format::YUVU =>
                            ((4 * self.config.parameters.width * self.config.parameters.height) / 2)
                                as usize,
                    }
                ])
                .wrap_err("failed to queue buffer")?;
        }
        self.camera = Some(camera);
        Ok(())
    }
}
