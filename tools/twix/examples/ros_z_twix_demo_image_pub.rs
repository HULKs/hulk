use std::time::Duration;

use color_eyre::eyre::{Result, eyre};
use ros_z::context::ContextBuilder;
use ros2::{builtin_interfaces::time::Time, sensor_msgs::image::Image, std_msgs::header::Header};

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let context = ContextBuilder::default()
        .build()
        .await
        .map_err(|error| eyre!(error.to_string()))?;
    let node = context
        .create_node("twix_demo_image_pub")
        .with_namespace("tools")
        .build()
        .await
        .map_err(|error| eyre!(error.to_string()))?;
    let publisher = node
        .publisher::<Image>("/twix_demo/image")
        .build()
        .await
        .map_err(|error| eyre!(error.to_string()))?;

    let mut tick = 0u8;
    loop {
        publisher
            .publish(&demo_image(tick))
            .await
            .map_err(|error| eyre!(error.to_string()))?;
        tick = tick.wrapping_add(1);
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

fn demo_image(tick: u8) -> Image {
    let width = 8;
    let height = 8;
    let mut data = Vec::with_capacity(width * height * 3);
    for y in 0..height {
        for x in 0..width {
            data.push((x as u8).wrapping_mul(32).wrapping_add(tick));
            data.push((y as u8).wrapping_mul(32));
            data.push(tick);
        }
    }

    Image {
        header: Header {
            stamp: Time { sec: 0, nanosec: 0 },
            frame_id: "twix_demo_camera".to_string(),
        },
        height: height as u32,
        width: width as u32,
        encoding: "rgb8".to_string(),
        is_bigendian: 0,
        step: (width * 3) as u32,
        data,
    }
}
