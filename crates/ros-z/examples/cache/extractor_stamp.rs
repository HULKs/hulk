//! Cache consumer — ExtractorStamp (application-level timestamp).
//!
//! The stamp extractor reads the message payload as `"msg-<seq>"` and maps it
//! to a logical [`ros_z::time::Time`]. In real sensor-fusion code you would read
//! `header.stamp` instead.
//!
//! ## Usage
//!
//! Terminal 1:
//! ```text
//! cargo run --example cache_talker
//! ```
//!
//! Terminal 2:
//! ```text
//! cargo run --example cache_extractor_stamp
//! ```

use std::time::Duration;

use ros_z::{Result, time::Time};
use ros_z_msgs::std_msgs::String as RosString;

pub async fn run(
    context: ros_z::context::Context,
    topic: String,
    capacity: usize,
    count: usize,
) -> Result<()> {
    let node = context.create_node("cache_consumer_app").build().await?;

    // Extractor reads the sequence number from the payload as logical seconds
    // since timeline zero. For real data: read header.stamp.
    let cache = node
        .create_cache::<RosString>(&topic, capacity)
        .with_stamp(|message: &RosString| {
            let secs: u64 = message
                .data
                .split('-')
                .next_back()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            Time::from_nanos((secs as i64) * 1_000_000_000)
        })
        .build()
        .await?;

    println!(
        "[cache/app] subscribed to '{}', capacity={} (app-level stamp)",
        topic, capacity,
    );

    tokio::time::sleep(Duration::from_millis(300)).await;

    let mut i = 0usize;
    loop {
        println!(
            "[cache/app] len={} | oldest={:?} | newest={:?}",
            cache.len(),
            cache.oldest_stamp(),
            cache.newest_stamp(),
        );

        // Find the message whose logical timestamp is closest to t=5s.
        let target = Time::from_nanos(5_000_000_000);
        let nearest = cache.get_nearest(target);
        println!(
            "[cache/app] nearest to t=5s: {}",
            nearest
                .as_ref()
                .map(|m| m.data.as_str())
                .unwrap_or("(none)"),
        );

        i += 1;
        if count > 0 && i >= count {
            break;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    Ok(())
}

#[cfg(not(test))]
#[tokio::main]
async fn main() -> Result<()> {
    zenoh::init_log_from_env_or("error");
    let context = ros_z::context::ContextBuilder::default().build().await?;
    run(context, "/cache_demo".into(), 20, 0).await
}
