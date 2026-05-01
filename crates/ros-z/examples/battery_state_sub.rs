use ros_z::{Result, context::ContextBuilder};
use ros_z_msgs::sensor_msgs::BatteryState;

#[tokio::main]
async fn main() -> Result<()> {
    let context = ContextBuilder::default().build().await?;
    let node = context
        .create_node("battery_state_subscriber")
        .build()
        .await?;
    let zsub = node
        .subscriber::<BatteryState>("battery_status")
        .build()
        .await?;

    println!("Listening for BatteryState messages on /battery_status...");

    loop {
        let received = zsub.recv_with_metadata().await?;
        println!("Received BatteryState:");
        println!("  Transport time: {:?}", received.transport_time);
        println!("  Source time: {:?}", received.source_time);
        println!("  Voltage: {:.2}V", received.voltage);
        println!("  Percentage: {:.1}%", received.percentage * 100.0);
        println!(
            "  Status: {}",
            match received.power_supply_status {
                BatteryState::POWER_SUPPLY_STATUS_UNKNOWN => "Unknown",
                BatteryState::POWER_SUPPLY_STATUS_CHARGING => "Charging",
                BatteryState::POWER_SUPPLY_STATUS_DISCHARGING => "Discharging",
                BatteryState::POWER_SUPPLY_STATUS_NOT_CHARGING => "Not Charging",
                BatteryState::POWER_SUPPLY_STATUS_FULL => "Full",
                _ => "Invalid",
            }
        );
        println!("  Temperature: {:.1}°C", received.temperature);
        println!("  Current: {:.2}A", received.current);
        println!("  Charge: {:.2}Ah", received.charge);
        println!("  Capacity: {:.2}Ah", received.capacity);
        println!("---");
    }
}
