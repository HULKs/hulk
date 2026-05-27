use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::{boxed::Box, future::Future, pin::Pin};
use std::{sync::Arc, time::Duration};

use color_eyre::Result;
use image::RgbImage;

use ros_z::{prelude::*, time::Time};
use ros2::{
    builtin_interfaces::time::Time as Ros2Time,
    sensor_msgs::{camera_info::CameraInfo, image::Image},
};
use types::{time_wrapper::TimeWrapper, ycbcr422_image::YCbCr422Image};
use x5_receiver::receiver::X5Receiver;

const X5_ADDRESS: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 127, 10)), 7654);

fn ros2_time_to_ros_z_time(time: Ros2Time) -> Time {
    let seconds = i64::from(time.sec).saturating_mul(1_000_000_000);
    let nanoseconds = i64::from(time.nanosec);
    Time::from_nanos(seconds.saturating_add(nanoseconds))
}

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("image_receiver").build().await?;

    let left_image_pub = node
        .publisher::<TimeWrapper<Image>>("inputs/left_image")?
        .build()
        .await?;
    let right_image_pub = node
        .publisher::<Image>("inputs/right_image")?
        .build()
        .await?;
    let camera_info_pub = node
        .publisher::<CameraInfo>("inputs/camera_info")?
        .build()
        .await?;
    let ycbcr422_image_pub = node
        .publisher::<TimeWrapper<YCbCr422Image>>("inputs/ycbcr422_image")?
        .build()
        .await?;

    let x5_receiver = X5Receiver::new(X5_ADDRESS);
    let left_camera_info = x5_receiver.last_camera_info().await.left_camera_info();
    let mut camera_info_timer = node.clock().timer(Duration::from_secs(1));

    loop {
        tokio::select! {
            left_frame = x5_receiver.next_left_frame() => {
                let left_image: Image = left_frame.into();
                let image_time = ros2_time_to_ros_z_time(left_image.header.stamp.clone());
                left_image_pub.publish(&TimeWrapper { time: image_time, inner: left_image.clone() }).await?;
                let rgb_image: RgbImage = left_image.try_into()?;
                ycbcr422_image_pub.publish(&TimeWrapper { time: image_time, inner: (&rgb_image).into() }).await?;
            }
            right_frame = x5_receiver.next_right_frame() => {
                right_image_pub.publish(&right_frame.into()).await?;
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

#[cfg(test)]
mod tests {
    use ros2::builtin_interfaces::time::Time as Ros2Time;

    use super::*;

    #[test]
    fn converts_ros2_header_stamp_to_ros_z_time() {
        let time = ros2_time_to_ros_z_time(Ros2Time {
            sec: 12,
            nanosec: 345,
        });

        assert_eq!(time.as_nanos(), 12_000_000_345);
    }
}
