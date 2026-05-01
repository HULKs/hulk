use std::time::Duration;

use ros_z::{Result, context::ContextBuilder};
use ros_z_msgs::{builtin_interfaces::Time, sensor_msgs::LaserScan, std_msgs::Header};

#[tokio::main]
async fn main() -> Result<()> {
    let context = ContextBuilder::default().build().await?;
    let node = context.create_node("laser_scan_publisher").build().await?;
    let publisher = node.publisher::<LaserScan>("scan").build().await?;

    let mut sequence = 0_u32;
    loop {
        let angle_min = -135.0_f32.to_radians();
        let angle_max = 135.0_f32.to_radians();
        let reading_count = 540;
        let angle_increment = (angle_max - angle_min) / (reading_count as f32 - 1.0);
        let mut ranges = Vec::with_capacity(reading_count);
        let mut intensities = Vec::with_capacity(reading_count);

        for index in 0..reading_count {
            let angle = angle_min + index as f32 * angle_increment;
            ranges.push(3.0 + 2.0 * angle.cos());
            intensities.push(100.0 + 50.0 * (index as f32 / reading_count as f32));
        }

        let message = LaserScan {
            header: Header {
                stamp: Time {
                    sec: (sequence / 10) as i32,
                    nanosec: (sequence % 10) * 100_000_000,
                },
                frame_id: "laser".to_string(),
            },
            angle_min,
            angle_max,
            angle_increment,
            time_increment: 0.0001,
            scan_time: 0.1,
            range_min: 0.1,
            range_max: 10.0,
            ranges,
            intensities,
        };

        publisher.publish(&message).await?;
        println!(
            "Published LaserScan #{sequence}: {} ranges",
            message.ranges.len()
        );
        sequence += 1;
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
