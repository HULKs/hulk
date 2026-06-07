use std::future::Future;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::pin::Pin;
use std::{sync::Arc, time::Duration};

use color_eyre::Result;

use ros_z::prelude::*;
use ros2::sensor_msgs::{camera_info::CameraInfo, image::Image};
use x5_receiver::receiver::X5Receiver;

const X5_ADDRESS: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 127, 10)), 7654);

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("image_receiver").build().await?;

    let left_image_pub = node
        .publisher::<Image>("inputs/left_image")?
        .build()
        .await?;
    let camera_info_pub = node
        .publisher::<CameraInfo>("inputs/camera_info")?
        .build()
        .await?;

    let x5_receiver = X5Receiver::new(X5_ADDRESS);
    let left_camera_info = x5_receiver.last_camera_info().await.left_camera_info();
    let mut camera_info_timer = node.clock().timer(Duration::from_secs(1));

    loop {
        tokio::select! {
            left_frame = x5_receiver.next_left_frame() => {
                left_image_pub.publish(&left_frame.into()).await?;
            }
            // TODO Make this either a service or a local transiert publisher
            _ = camera_info_timer.tick() => {
                camera_info_pub
                    .publish(&left_camera_info)
                    .await?;
            }
        }
    }
}
