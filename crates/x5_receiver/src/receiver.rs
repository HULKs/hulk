use std::{io, net::SocketAddr, time::Duration};

use async_channel::{Receiver, Sender};
use futures_util::TryFutureExt;
use tokio::{io::AsyncReadExt, net::TcpStream, task::JoinHandle};

use crate::types::{
    MAGIC_IDENTIFIER_CAMERA_INFO, MAGIC_IDENTIFIER_FRAME, X5CameraFrame, X5CameraFrameHeader,
    X5CameraInfo,
};

pub const MAX_ALLOCATION_SIZE: usize = 4 * 1024 * 1024;
pub const EXPOSURE_TIME: Duration = Duration::from_millis(12);

pub struct X5Receiver {
    pub left: X5ImageReceiver,
    pub right: X5ImageReceiver,
    pub camera_info: X5CameraInfoReceiver,
    pub task: JoinHandle<()>,
}

pub struct X5ImageReceiver {
    frame_receiver: Receiver<X5CameraFrame>,
}

pub struct X5CameraInfoReceiver {
    camera_info_receiver: Receiver<X5CameraInfo>,
}

impl X5Receiver {
    /// Starts receiving frames and calibration until all receiving endpoints are dropped.
    pub fn new(address: SocketAddr) -> Self {
        let (left_frame_sender, left_frame_receiver) = async_channel::bounded(1);
        let (right_frame_sender, right_frame_receiver) = async_channel::bounded(1);
        let (camera_info_sender, camera_info_receiver) = async_channel::bounded(1);
        let task = tokio::spawn(restarting_x5_receiver_task(
            address,
            left_frame_sender,
            right_frame_sender,
            camera_info_sender,
        ));
        Self {
            left: X5ImageReceiver {
                frame_receiver: left_frame_receiver,
            },
            right: X5ImageReceiver {
                frame_receiver: right_frame_receiver,
            },
            camera_info: X5CameraInfoReceiver {
                camera_info_receiver,
            },
            task,
        }
    }
}

impl X5CameraInfoReceiver {
    /// Receives the latest unread calibration.
    pub async fn recv(&mut self) -> X5CameraInfo {
        self.camera_info_receiver
            .recv()
            .await
            .expect("x5 receiver must be running")
    }
}

impl X5ImageReceiver {
    pub async fn recv(&mut self) -> X5CameraFrame {
        self.frame_receiver
            .recv()
            .await
            .expect("x5 receiver must be running")
    }
}

async fn restarting_x5_receiver_task(
    address: SocketAddr,
    left_frame_sender: Sender<X5CameraFrame>,
    right_frame_sender: Sender<X5CameraFrame>,
    camera_info_sender: Sender<X5CameraInfo>,
) {
    const RETRY_INTERVAL: Duration = Duration::from_secs(5);
    tokio::select! {
        _ = async {
            tokio::join!(
                left_frame_sender.closed(),
                right_frame_sender.closed(),
                camera_info_sender.closed(),
            );
        } => {}
        _ = async {
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
        } => {}
    }
}

struct X5ReceiverTask {
    connection: TcpStream,
    left_frame_sender: Sender<X5CameraFrame>,
    right_frame_sender: Sender<X5CameraFrame>,
    camera_info_sender: Sender<X5CameraInfo>,
}

impl X5ReceiverTask {
    async fn connect(
        address: SocketAddr,
        left_frame_sender: Sender<X5CameraFrame>,
        right_frame_sender: Sender<X5CameraFrame>,
        camera_info_sender: Sender<X5CameraInfo>,
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
                    // Keep reading the shared stream if a camera receiver is dropped.
                    match camera_frame.header.channel {
                        0 => {
                            let _ = self.left_frame_sender.force_send(camera_frame);
                        }
                        1 => {
                            let _ = self.right_frame_sender.force_send(camera_frame);
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
                    let _ = self.camera_info_sender.force_send(camera_info);
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

        let mut header: X5CameraFrameHeader = unsafe { std::mem::transmute(header_bytes) };
        if header.payload_size as usize > MAX_ALLOCATION_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "payload size exceeds maximum allocation size",
            ));
        }
        header.timestamp_nanoseconds += EXPOSURE_TIME.as_nanos() as u64 / 2;

        let mut nv12_data = vec![0; header.payload_size as usize];
        self.connection.read_exact(&mut nv12_data).await?;

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

        Ok(camera_information)
    }
}

#[cfg(test)]
mod tests {
    use std::pin::pin;

    use futures_util::poll;
    use tokio::{io::AsyncWriteExt, net::TcpListener, time::timeout};

    use super::*;

    async fn send_frame(connection: &mut TcpStream, channel: u8, frame_identifier: u32) {
        connection
            .write_u32_le(MAGIC_IDENTIFIER_FRAME)
            .await
            .unwrap();
        connection.write_u8(channel).await.unwrap();
        connection.write_u32_le(frame_identifier).await.unwrap();
        connection.write_u64_le(1_000_000_000).await.unwrap();
        connection.write_u16_le(2).await.unwrap();
        connection.write_u16_le(2).await.unwrap();
        connection.write_u32_le(6).await.unwrap();
        connection
            .write_all(&[frame_identifier as u8; 6])
            .await
            .unwrap();
    }

    async fn send_camera_info(connection: &mut TcpStream, width: u16) {
        connection
            .write_u32_le(MAGIC_IDENTIFIER_CAMERA_INFO)
            .await
            .unwrap();
        connection.write_u32_le(490).await.unwrap();
        let mut payload = [0; 490];
        payload[..2].copy_from_slice(&width.to_le_bytes());
        connection.write_all(&payload).await.unwrap();
    }

    #[tokio::test]
    async fn delivers_only_latest_unread_frames_without_repeating_consumed_frames() {
        timeout(Duration::from_secs(10), async {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let X5Receiver {
                mut left,
                mut right,
                mut camera_info,
                task,
            } = X5Receiver::new(listener.local_addr().unwrap());
            let (mut connection, _) = listener.accept().await.unwrap();

            send_frame(&mut connection, 0, 1).await;
            send_frame(&mut connection, 1, 10).await;
            send_frame(&mut connection, 0, 2).await;
            send_frame(&mut connection, 1, 11).await;
            // Calibration follows the frames on the same stream, so waiting for it also
            // ensures acquisition has processed every frame without waiting for capacity.
            send_camera_info(&mut connection, 640).await;
            camera_info.recv().await;

            assert_eq!(left.recv().await.nv12_data, [2; 6]);
            assert_eq!(right.recv().await.nv12_data, [11; 6]);
            // Poll and cancel exactly as an unselected tokio::select! branch would.
            assert!(poll!(pin!(left.recv())).is_pending());
            assert!(poll!(pin!(right.recv())).is_pending());

            send_frame(&mut connection, 0, 3).await;
            send_frame(&mut connection, 1, 12).await;
            assert_eq!(left.recv().await.nv12_data, [3; 6]);
            assert_eq!(right.recv().await.nv12_data, [12; 6]);

            drop((left, right, camera_info));
            task.await.unwrap();
        })
        .await
        .expect("frame delivery must not stall acquisition");
    }

    #[tokio::test]
    async fn dropped_camera_does_not_interrupt_other_camera_or_calibration() {
        timeout(Duration::from_secs(10), async {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let X5Receiver {
                left,
                mut right,
                mut camera_info,
                task,
            } = X5Receiver::new(listener.local_addr().unwrap());
            let (mut connection, _) = listener.accept().await.unwrap();
            drop(left);

            send_frame(&mut connection, 0, 1).await;
            send_frame(&mut connection, 1, 10).await;
            send_frame(&mut connection, 0, 2).await;
            send_camera_info(&mut connection, 640).await;

            let width = camera_info.recv().await.width;
            assert_eq!(width, 640);
            assert_eq!(right.recv().await.nv12_data, [10; 6]);

            send_camera_info(&mut connection, 800).await;
            send_frame(&mut connection, 1, 11).await;
            let width = camera_info.recv().await.width;
            assert_eq!(width, 800);
            assert_eq!(right.recv().await.nv12_data, [11; 6]);

            drop((right, camera_info));
            task.await.unwrap();
            assert_eq!(connection.read(&mut [0]).await.unwrap(), 0);
        })
        .await
        .expect("dropping one camera must not interrupt the shared reader");
    }

    #[tokio::test]
    async fn calibration_interest_keeps_reading_until_dropped() {
        timeout(Duration::from_secs(10), async {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let X5Receiver {
                left,
                right,
                mut camera_info,
                task,
            } = X5Receiver::new(listener.local_addr().unwrap());
            let (mut connection, _) = listener.accept().await.unwrap();
            drop(left);
            drop(right);

            assert!(poll!(pin!(camera_info.recv())).is_pending());
            send_frame(&mut connection, 0, 1).await;
            send_frame(&mut connection, 1, 10).await;
            send_camera_info(&mut connection, 640).await;
            let width = camera_info.recv().await.width;
            assert_eq!(width, 640);
            assert!(poll!(pin!(camera_info.recv())).is_pending());

            drop(camera_info);
            task.await.unwrap();
        })
        .await
        .expect("calibration must keep the shared reader alive until dropped");
    }
}
