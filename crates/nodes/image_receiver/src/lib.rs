use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::{sync::Arc, time::Duration};

use color_eyre::Result;
use image::RgbImage;

use ros_z::{IntoEyreResultExt, prelude::*};
use ros2::sensor_msgs::{camera_info::CameraInfo, image::Image};
use types::ycbcr422_image::YCbCr422Image;
use x5_receiver::receiver::X5Receiver;

const X5_ADDRESS: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 127, 10)), 7654);

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("image_receiver")
        .build()
        .await
        .into_eyre()?;

    let left_image_pub = node
        .publisher::<Image>("inputs/left_image")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let right_image_pub = node
        .publisher::<Image>("inputs/left_image")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let camera_info_pub = node
        .publisher::<CameraInfo>("inputs/camera_info")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let ycbcr422_image_pub = node
        .publisher::<YCbCr422Image>("inputs/ycbcr422_image")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    let x5_receiver = X5Receiver::new(X5_ADDRESS);
    let left_camera_info = x5_receiver.last_camera_info().await.left_camera_info();
    let mut camera_info_timer = node.clock().timer(Duration::from_secs(1));

    loop {
        tokio::select! {
            left_frame = x5_receiver.next_left_frame() => {
                let left_image: Image = left_frame.into();
                left_image_pub.publish(&left_image).await.into_eyre()?;
                let rgb_image: RgbImage = left_image.try_into()?;
                ycbcr422_image_pub.publish(&(&rgb_image).into()).await.into_eyre()?;
            }
            right_frame = x5_receiver.next_right_frame() => {
                right_image_pub.publish(&right_frame.into()).await.into_eyre()?;
            }
            // TODO Make this either a service or a local transiert publisher
            _ = camera_info_timer.tick() => {
                camera_info_pub
                    .publish(&left_camera_info)
                    .await
                    .into_eyre()?;
            }
        }
    }
}
