use std::{sync::Arc, time::Duration};

use color_eyre::Result;
use ros_z::{prelude::*, time::Time};
use ros2::{
    builtin_interfaces::time::Time as Ros2Time, sensor_msgs::image::Image, std_msgs::header::Header,
};
use types::ycbcr422_image::YCbCr422Image;

use crate::IntoEyreResultExt;

const FAKE_IMAGE_WIDTH: u32 = 64;
const FAKE_IMAGE_HEIGHT: u32 = 48;
const FAKE_IMAGE_PERIOD: Duration = Duration::from_millis(100);

fn fake_image(frame_index: u32, stamp: Time) -> Image {
    let mut data = Vec::with_capacity((FAKE_IMAGE_WIDTH * FAKE_IMAGE_HEIGHT * 3) as usize);

    for y in 0..FAKE_IMAGE_HEIGHT {
        for x in 0..FAKE_IMAGE_WIDTH {
            let phase = frame_index.wrapping_mul(3);
            data.push((x as u8).wrapping_mul(4).wrapping_add(phase as u8));
            data.push((y as u8).wrapping_mul(5).wrapping_add((phase / 2) as u8));
            data.push(((x + y + phase) % 256) as u8);
        }
    }

    Image {
        header: Header {
            stamp: Ros2Time::from(stamp.to_wallclock()),
            frame_id: "fake_camera".to_string(),
        },
        height: FAKE_IMAGE_HEIGHT,
        width: FAKE_IMAGE_WIDTH,
        encoding: "rgb8".to_string(),
        is_bigendian: 0,
        step: FAKE_IMAGE_WIDTH * 3,
        data,
    }
}

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx
        .create_node("image_receiver")
        .build()
        .await
        .into_eyre()?;
    let image_pub = node
        .publisher::<Image>("image")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;
    let ycbcr422_image_pub = node
        .publisher::<YCbCr422Image>("ycbcr422_image")
        .into_eyre()?
        .build()
        .await
        .into_eyre()?;

    let mut timer = node.create_timer(FAKE_IMAGE_PERIOD);
    let mut frame_index = 0u32;

    loop {
        let stamp = timer.tick().await;

        let image = fake_image(frame_index, stamp);
        let ycbcr422_image = YCbCr422Image::try_from(&image)?;

        image_pub.publish(&image).await.into_eyre()?;
        ycbcr422_image_pub
            .publish(&ycbcr422_image)
            .await
            .into_eyre()?;

        frame_index = frame_index.wrapping_add(1);
    }
}
