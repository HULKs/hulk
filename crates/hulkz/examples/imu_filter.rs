//! Simple IMU Filter Example
//!
//! Demonstrates a minimal node that:
//! - Subscribes to raw IMU data
//! - Applies a low-pass filter
//! - Publishes filtered data with the source timestamp
//!
//! This example shows:
//! - Clean subscribe -> process -> publish pattern
//! - Explicit timestamp propagation (filtered data inherits source timestamp)
//! - Delta time tracking for time-aware filters
//!
//! Run with: `cargo run --example imu_filter`

use std::time::Duration;

use hulkz::{Result, Session, Timestamp};
use serde::{Deserialize, Serialize};

/// IMU sensor reading
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Imu {
    /// Linear acceleration [x, y, z] in m/s^2
    accel: [f64; 3],
    /// Angular velocity [x, y, z] in rad/s
    gyro: [f64; 3],
}

/// Simple exponential moving average filter
struct LowPassFilter {
    alpha: f64,
    state: Option<Imu>,
}

impl LowPassFilter {
    fn new(alpha: f64) -> Self {
        Self { alpha, state: None }
    }

    fn update(&mut self, input: &Imu, _dt: Duration) -> Imu {
        match &self.state {
            None => {
                self.state = Some(input.clone());
                input.clone()
            }
            Some(prev) => {
                let filtered = Imu {
                    accel: [
                        self.alpha * input.accel[0] + (1.0 - self.alpha) * prev.accel[0],
                        self.alpha * input.accel[1] + (1.0 - self.alpha) * prev.accel[1],
                        self.alpha * input.accel[2] + (1.0 - self.alpha) * prev.accel[2],
                    ],
                    gyro: [
                        self.alpha * input.gyro[0] + (1.0 - self.alpha) * prev.gyro[0],
                        self.alpha * input.gyro[1] + (1.0 - self.alpha) * prev.gyro[1],
                        self.alpha * input.gyro[2] + (1.0 - self.alpha) * prev.gyro[2],
                    ],
                };
                self.state = Some(filtered.clone());
                filtered
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let session = Session::create("robot").await?;
    println!("Session created: {}", session.id());

    // Spawn a simulated IMU publisher
    spawn_imu_publisher(session.clone());
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create the filter node
    let node = session.create_node("imu_filter").build().await?;

    let mut imu_sub = node.subscribe::<Imu>("imu/raw").build().await?;
    let imu_pub = node.advertise::<Imu>("imu/filtered").build().await?;

    println!("Subscribed to: imu/raw");
    println!("Publishing to: imu/filtered");
    println!("Running filter... (Ctrl+C to stop)\n");

    let mut filter = LowPassFilter::new(0.1);
    let mut last_timestamp: Option<Timestamp> = None;

    loop {
        let msg = imu_sub.recv_async().await?;

        // Calculate delta time since last message
        let dt = last_timestamp
            .as_ref()
            .map(|last| msg.timestamp.get_diff_duration(last))
            .unwrap_or(Duration::ZERO);
        last_timestamp = Some(msg.timestamp.clone());

        // Apply filter
        let filtered = filter.update(&msg.payload, dt);

        // Publish with source timestamp (filtered data inherits IMU timestamp)
        imu_pub.put(&filtered, &msg.timestamp).await?;

        println!(
            "dt={:>5.1}ms  raw=[{:>6.2}, {:>6.2}, {:>6.2}]  filtered=[{:>6.2}, {:>6.2}, {:>6.2}]",
            dt.as_secs_f64() * 1000.0,
            msg.payload.accel[0],
            msg.payload.accel[1],
            msg.payload.accel[2],
            filtered.accel[0],
            filtered.accel[1],
            filtered.accel[2],
        );
    }
}

/// Spawns a task that publishes simulated noisy IMU data at ~100 Hz
fn spawn_imu_publisher(session: Session) {
    tokio::spawn(async move {
        let node = session.create_node("imu_sensor").build().await.unwrap();
        let publisher = node.advertise::<Imu>("imu/raw").build().await.unwrap();

        let mut t = 0.0f64;
        loop {
            // Simulate noisy sensor data using deterministic oscillations
            let noise = |phase: f64| (t * 17.3 + phase).sin() * 0.3;
            let imu = Imu {
                accel: [noise(0.0), noise(1.0), 9.81 + noise(2.0)],
                gyro: [0.01 * t.sin() + noise(3.0) * 0.1, noise(4.0) * 0.1, noise(5.0) * 0.1],
            };
            publisher.put(&imu, &session.now()).await.unwrap();
            t += 0.1;
            tokio::time::sleep(Duration::from_millis(10)).await; // 100 Hz
        }
    });
}
