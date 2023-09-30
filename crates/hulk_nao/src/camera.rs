use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::sync::watch::{self, Receiver, Sender};

use color_eyre::{
    eyre::{bail, Context},
    Result,
};
use nao_camera::{reset_camera_device, Camera as NaoCamera, Parameters, PollingError};
use parking_lot::{Mutex, RwLock};
use types::{camera_position::CameraPosition, ycbcr422_image::YCbCr422Image};

pub struct Camera {
    camera: RwLock<Option<NaoCamera>>,
    path: PathBuf,
    camera_position: CameraPosition,
    read_mutex: Mutex<()>,
    parameters: Parameters,
    i2c_head_mutex: Arc<Mutex<()>>,
    image_sender: Sender<Option<YCbCr422Image>>,
    image_receiver: Receiver<Option<YCbCr422Image>>,
}

impl Camera {
    pub fn new(
        path: impl AsRef<Path>,
        camera_position: CameraPosition,
        parameters: Parameters,
        i2c_head_mutex: Arc<Mutex<()>>,
    ) -> Result<Self> {
        let (sender, receiver) = watch::channel(None);
        let camera = Self {
            camera: RwLock::new(None),
            path: path.as_ref().to_path_buf(),
            camera_position,
            read_mutex: Mutex::new(()),
            parameters,
            i2c_head_mutex,
            image_sender: sender,
            image_receiver: receiver,
        };
        camera.reset().wrap_err("failed to reset")?;
        Ok(camera)
    }

    pub fn read(&self) -> Result<YCbCr422Image> {
        let mut this_receiver = self.image_receiver.clone();
        println!("New receiver");
        this_receiver.borrow_and_update();
        println!("Receiver empty");
        if let Some(_lock) = self.read_mutex.try_lock() {
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
            self.image_sender.send(Some(new_image.clone()))?;
            return Ok(new_image);
        }
        let image = this_receiver.borrow_and_update().clone();
        Ok(image.unwrap())
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
