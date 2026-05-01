//! Cache talker — publishes a string message every 100 ms.
//!
//! ## Usage
//!
//! ```text
//! cargo run --example cache_talker
//! ```
//!
//! Run alongside [`cache_zenoh_stamp`] or [`cache_extractor_stamp`] to see the
//! cache fill up.

use std::time::Duration;

use ros_z::Result;
use ros_z_msgs::std_msgs::String as RosString;

pub async fn run(context: ros_z::context::Context, topic: String, count: usize) -> Result<()> {
    let node = context.create_node("cache_talker").build().await?;
    let publisher = node.publisher::<RosString>(&topic).build().await?;

    println!("[talker] publishing on '{}' every 100 ms", topic);

    let mut seq: u64 = 0;
    loop {
        let message = RosString {
            data: format!("msg-{seq}"),
        };
        publisher.publish(&message).await?;
        println!("[talker] sent: {}", message.data);
        tokio::time::sleep(Duration::from_millis(100)).await;
        seq += 1;
        if count > 0 && seq as usize >= count {
            break;
        }
    }
    Ok(())
}

#[cfg(not(test))]
#[tokio::main]
async fn main() -> Result<()> {
    zenoh::init_log_from_env_or("error");
    let context = ros_z::context::ContextBuilder::default().build().await?;
    run(context, "/cache_demo".into(), 0).await
}
