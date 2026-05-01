use std::time::{Duration, Instant};

use ros_z::{Result, ZBuf, context::ContextBuilder};
use ros_z_msgs::std_msgs::ByteMultiArray;
use zenoh_buffers::buffer::SplitBuffer;

const PAYLOAD_BYTES: usize = 64;
const SAMPLE_COUNT: usize = 100;
const FREQUENCY_HZ: f64 = 10.0;

fn percentile(data: &[u64], percentile: f64) -> u64 {
    let index = ((percentile * data.len() as f64).round() as usize).min(data.len() - 1);
    data[index]
}

fn print_statistics(mut rtts: Vec<u64>) {
    if rtts.is_empty() {
        println!("No RTT samples collected");
        return;
    }

    rtts.sort();
    println!("RTT stats in nanoseconds:");
    println!("min: {}", rtts[0]);
    println!("p50: {}", percentile(&rtts, 0.50));
    println!("p95: {}", percentile(&rtts, 0.95));
    println!("max: {}", rtts[rtts.len() - 1]);
}

#[tokio::main]
async fn main() -> Result<()> {
    let context = ContextBuilder::default()
        .with_logging_enabled()
        .build()
        .await?;
    let node = context.create_node("ping_node").build().await?;
    let publisher = node.publisher::<ByteMultiArray>("ping").build().await?;
    let subscriber = node.subscriber::<ByteMultiArray>("pong").build().await?;
    let period = Duration::from_secs_f64(1.0 / FREQUENCY_HZ);
    let start = Instant::now();
    let mut rtts = Vec::with_capacity(SAMPLE_COUNT);

    println!(
        "Sending {SAMPLE_COUNT} ping samples with {PAYLOAD_BYTES} byte payloads at {FREQUENCY_HZ} Hz"
    );

    while rtts.len() < SAMPLE_COUNT {
        let sent_time = start.elapsed().as_nanos() as u64;
        let mut buffer = vec![0xAA; PAYLOAD_BYTES];
        buffer[0..8].copy_from_slice(&sent_time.to_le_bytes());

        let message = ByteMultiArray {
            data: ZBuf::from(buffer),
            ..Default::default()
        };
        publisher.publish(&message).await?;

        let reply = subscriber.recv().await?;
        let bytes = reply.data.contiguous();
        if bytes.len() < 8 {
            println!("Ignoring short pong payload: {} bytes", bytes.len());
            continue;
        }

        let mut timestamp = [0_u8; 8];
        timestamp.copy_from_slice(&bytes[0..8]);
        let original_time = u64::from_le_bytes(timestamp);
        rtts.push(start.elapsed().as_nanos() as u64 - original_time);

        tokio::time::sleep(period).await;
    }

    print_statistics(rtts);
    Ok(())
}
