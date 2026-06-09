use std::{io, net::SocketAddr, sync::Arc, time::Duration};

use futures_util::TryFutureExt;
use tokio::{io::AsyncReadExt, net::TcpStream, sync::watch};
use tokio_util::task::AbortOnDropHandle;

use crate::types::{
    DISTORTION_COEFFICIENT_COUNT, DistortionCoefficientCount, MAGIC_IDENTIFIER_CAMERA_INFO,
    MAGIC_IDENTIFIER_FRAME, X5CameraFrame, X5CameraFrameHeader, X5CameraInfo,
};

pub const MAX_ALLOCATION_SIZE: usize = 4 * 1024 * 1024;

pub struct X5Receiver {
    _task: AbortOnDropHandle<()>,
    left_frame_sender: watch::Sender<Option<X5CameraFrame>>,
    right_frame_sender: watch::Sender<Option<X5CameraFrame>>,
    camera_info_sender: watch::Sender<Option<X5CameraInfo>>,
}

pub struct X5ImageReceiver {
    frame_receiver: watch::Receiver<Option<X5CameraFrame>>,
}

pub enum Side {
    Left,
    Right,
}

impl X5Receiver {
    pub fn new(address: SocketAddr) -> Self {
        let (left_frame_sender, _) = watch::channel(None);
        let (right_frame_sender, _) = watch::channel(None);
        let (camera_info_sender, _) = watch::channel(None);
        let task = AbortOnDropHandle::new(tokio::spawn(restarting_x5_receiver_task(
            address,
            left_frame_sender.clone(),
            right_frame_sender.clone(),
            camera_info_sender.clone(),
        )));
        Self {
            _task: task,
            left_frame_sender,
            right_frame_sender,
            camera_info_sender,
        }
    }

    pub fn subscribe_image(&self, side: Side) -> X5ImageReceiver {
        let sender = match side {
            Side::Left => &self.left_frame_sender,
            Side::Right => &self.right_frame_sender,
        };

        X5ImageReceiver {
            frame_receiver: sender.subscribe(),
        }
    }

    pub async fn wait_for_camera_info(&self) -> X5CameraInfo {
        let mut receiver = self.camera_info_sender.subscribe();
        receiver
            .wait_for(|value| value.is_some())
            .await
            .expect("x5 receiver must be running")
            .expect("camera info must not be None")
    }
}

impl X5ImageReceiver {
    pub async fn recv(&mut self) -> X5CameraFrame {
        self.frame_receiver
            .changed()
            .await
            .expect("x5 receiver must be running");
        self.frame_receiver
            .borrow_and_update()
            .clone()
            .expect("image must not be None")
    }
}

async fn restarting_x5_receiver_task(
    address: SocketAddr,
    left_frame_sender: watch::Sender<Option<X5CameraFrame>>,
    right_frame_sender: watch::Sender<Option<X5CameraFrame>>,
    camera_info_sender: watch::Sender<Option<X5CameraInfo>>,
) {
    const RETRY_INTERVAL: Duration = Duration::from_secs(5);
    loop {
        let result = X5ReceiverTask::connect(
            address,
            left_frame_sender.clone(),
            right_frame_sender.clone(),
            camera_info_sender.clone(),
        )
        .and_then(async |connection| connection.run().await)
        .await;

        if let Err(error) = result {
            log::error!("x5 receiver error: {}", error);
            tokio::time::sleep(RETRY_INTERVAL).await;
            log::info!("reconnecting to camera");
        }
    }
}

struct X5ReceiverTask {
    connection: TcpStream,
    left_frame_sender: watch::Sender<Option<X5CameraFrame>>,
    right_frame_sender: watch::Sender<Option<X5CameraFrame>>,
    camera_info_sender: watch::Sender<Option<X5CameraInfo>>,
}

impl X5ReceiverTask {
    async fn connect(
        address: SocketAddr,
        left_frame_sender: watch::Sender<Option<X5CameraFrame>>,
        right_frame_sender: watch::Sender<Option<X5CameraFrame>>,
        camera_info_sender: watch::Sender<Option<X5CameraInfo>>,
    ) -> io::Result<Self> {
        let connection = TcpStream::connect(address).await?;

        Ok(Self {
            connection,
            left_frame_sender,
            right_frame_sender,
            camera_info_sender,
        })
    }

    async fn run(mut self) -> io::Result<()> {
        loop {
            let magic = self.connection.read_u32_le().await?;
            match magic {
                MAGIC_IDENTIFIER_FRAME => {
                    let camera_frame = self.receive_frame().await?;
                    match camera_frame.header.channel {
                        0 => {
                            self.left_frame_sender.send_replace(Some(camera_frame));
                        }
                        1 => {
                            self.right_frame_sender.send_replace(Some(camera_frame));
                        }
                        _ => {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "Invalid channel",
                            ));
                        }
                    }
                }
                MAGIC_IDENTIFIER_CAMERA_INFO => {
                    let camera_info = self.receive_camera_info().await?;
                    self.camera_info_sender.send_replace(Some(camera_info));
                }
                _ => {}
            }
        }
    }

    #[cfg(target_endian = "big")]
    compile_error!(
        "A little-endian target architecture is required because the network byte stream represents memory dumps from a little-endian X5 board."
    );
    async fn receive_frame(&mut self) -> io::Result<X5CameraFrame> {
        const { assert!(size_of::<X5CameraFrameHeader>() == 21) }
        let mut header_bytes = [0u8; size_of::<X5CameraFrameHeader>()];
        self.connection.read_exact(&mut header_bytes).await?;

        let header: X5CameraFrameHeader = unsafe { std::mem::transmute(header_bytes) };
        if header.payload_size as usize > MAX_ALLOCATION_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "payload size exceeds maximum allocation size",
            ));
        }

        let nv12_data = Arc::<[u8]>::new_zeroed_slice(header.payload_size as usize);
        let mut nv12_data = unsafe { nv12_data.assume_init() };

        self.connection
            .read_exact(Arc::get_mut(&mut nv12_data).expect("no other reference"))
            .await?;

        Ok(X5CameraFrame { header, nv12_data })
    }

    #[cfg(target_endian = "big")]
    compile_error!(
        "A little-endian target architecture is required because the network byte stream represents memory dumps from a little-endian X5 board."
    );
    async fn receive_camera_info(&mut self) -> io::Result<X5CameraInfo> {
        const { assert!(size_of::<X5CameraInfo>() == 490) }
        let length = self.connection.read_u32_le().await?;
        let mut payload_data = [0u8; size_of::<X5CameraInfo>()];
        if length != payload_data.len() as u32 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unexpected calibration payload length",
            ));
        }
        self.connection.read_exact(&mut payload_data).await?;
        let camera_information =
            unsafe { std::mem::transmute::<[u8; 490], X5CameraInfo>(payload_data) };
        validate_camera_info(&camera_information)?;

        Ok(camera_information)
    }
}

fn validate_camera_info(camera_info: &X5CameraInfo) -> io::Result<()> {
    validate_distortion_count("left", camera_info.distortion_count_left)?;
    validate_distortion_count("right", camera_info.distortion_count_right)
}

fn validate_distortion_count(side: &str, count: DistortionCoefficientCount) -> io::Result<()> {
    if !count.is_valid() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{side} distortion count {} exceeds {DISTORTION_COEFFICIENT_COUNT}",
                count.get()
            ),
        ));
    }

    Ok(())
}
