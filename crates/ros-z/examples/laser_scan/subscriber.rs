use ros_z::{Result, context::ContextBuilder};
use ros_z_msgs::sensor_msgs::LaserScan;

#[tokio::main]
async fn main() -> Result<()> {
    let context = ContextBuilder::default().build().await?;
    let node = context.create_node("laser_scan_subscriber").build().await?;
    let subscriber = node.subscriber::<LaserScan>("scan").build().await?;

    println!("Listening for LaserScan messages on /scan...");
    loop {
        let received = subscriber.recv_with_metadata().await?;
        println!(
            "Received LaserScan frame={} ranges={} transport={:?}",
            received.header.frame_id,
            received.ranges.len(),
            received.transport_time
        );
    }
}
