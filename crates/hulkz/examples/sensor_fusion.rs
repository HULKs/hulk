//! Sensor Fusion Example
//!
//! Demonstrates temporal data alignment using buffered subscriptions.
//!
//! This is a common robotics pattern where:
//! - A main topic (camera) triggers processing
//! - Data from other topics (IMU, odometry) is looked up at the trigger's timestamp
//!
//! Rates:
//! - Camera: ~10 Hz (main trigger)
//! - IMU: ~100 Hz (buffered for lookup)
//! - Odometry: ~50 Hz (buffered for lookup)
//!
//! When a camera frame arrives, the fusion node looks up the IMU and
//! odometry readings closest to the camera's timestamp.
//!
//! Run with: `cargo run --example sensor_fusion`

use std::time::Duration;

use hulkz::{Result, Session};
use serde::{Deserialize, Serialize};

/// Simulated camera image
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Image {
    width: u32,
    height: u32,
    seq: u32,
}

/// Simulated IMU reading
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Imu {
    /// Linear acceleration [x, y, z] in m/s^2
    accel: [f64; 3],
    /// Angular velocity [x, y, z] in rad/s
    gyro: [f64; 3],
}

/// Simulated odometry
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Odometry {
    x: f64,
    y: f64,
    theta: f64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let session = Session::create("fusion_demo").await?;
    println!("Session created: {}", session.id());

    // Spawn simulated sensor publishers
    spawn_camera_publisher(session.clone());
    spawn_imu_publisher(session.clone());
    spawn_odometry_publisher(session.clone());

    // Give publishers time to start and advertise
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create the fusion node
    let node = session.create_node("fusion").build().await?;
    println!("Fusion node created");

    // Main trigger: camera (direct subscription, not buffered)
    let mut camera = node.subscribe::<Image>("camera/image").build().await?;

    // Buffered subscriptions for temporal lookup
    let (imu_buffer, driver) = node.buffer::<Imu>("imu/data", 200).await?;
    tokio::spawn(driver);

    let (odom_buffer, driver) = node.buffer::<Odometry>("odometry", 100).await?;
    tokio::spawn(driver);

    println!("Subscribed to camera/image, imu/data, odometry");
    println!("Running sensor fusion... (Ctrl+C to stop)\n");

    // Main processing loop: triggered by camera frames
    loop {
        let frame = camera.recv_async().await?;
        let timestamp = frame.timestamp;

        // Look up IMU and odometry at the camera frame's timestamp
        let imu = imu_buffer.lookup_nearest(&timestamp).await;
        let odom = odom_buffer.lookup_nearest(&timestamp).await;

        // Print aligned data
        print!(
            "[Frame {:>4}] ",
            frame.payload.seq
        );

        if let Some(imu) = imu {
            let dt = timestamp.get_diff_duration(&imu.timestamp);
            print!(
                "IMU(dt={:>4}us, accel=[{:>6.2}, {:>6.2}, {:>6.2}]) ",
                dt.as_micros(),
                imu.payload.accel[0],
                imu.payload.accel[1],
                imu.payload.accel[2]
            );
        } else {
            print!("IMU(--) ");
        }

        if let Some(odom) = odom {
            let dt = timestamp.get_diff_duration(&odom.timestamp);
            print!(
                "Odom(dt={:>4}us, x={:>6.2}, y={:>6.2}, θ={:>5.2})",
                dt.as_micros(),
                odom.payload.x,
                odom.payload.y,
                odom.payload.theta
            );
        } else {
            print!("Odom(--)");
        }

        println!();
    }
}

/// Spawns a task that publishes simulated camera images at ~10 Hz
fn spawn_camera_publisher(session: Session) {
    tokio::spawn(async move {
        let node = session.create_node("camera").build().await.unwrap();
        let publisher = node
            .advertise::<Image>("camera/image")
            .build()
            .await
            .unwrap();

        let mut seq = 0u32;
        loop {
            let image = Image {
                width: 640,
                height: 480,
                seq,
            };
            publisher.put(&image, &session.now()).await.unwrap();
            seq = seq.wrapping_add(1);
            tokio::time::sleep(Duration::from_millis(100)).await; // 10 Hz
        }
    });
}

/// Spawns a task that publishes simulated IMU data at ~100 Hz
fn spawn_imu_publisher(session: Session) {
    tokio::spawn(async move {
        let node = session.create_node("imu").build().await.unwrap();
        let publisher = node.advertise::<Imu>("imu/data").build().await.unwrap();

        let mut t = 0.0f64;
        loop {
            // Simulate some sensor data with slight oscillation
            let imu = Imu {
                accel: [0.1 * t.sin(), 0.1 * t.cos(), 9.81],
                gyro: [0.01 * t.cos(), 0.01 * t.sin(), 0.0],
            };
            publisher.put(&imu, &session.now()).await.unwrap();
            t += 0.1;
            tokio::time::sleep(Duration::from_millis(10)).await; // 100 Hz
        }
    });
}

/// Spawns a task that publishes simulated odometry at ~50 Hz
fn spawn_odometry_publisher(session: Session) {
    tokio::spawn(async move {
        let node = session.create_node("odom").build().await.unwrap();
        let publisher = node
            .advertise::<Odometry>("odometry")
            .build()
            .await
            .unwrap();

        let mut theta = 0.0f64;
        loop {
            // Simulate circular motion
            theta += 0.02;
            let odom = Odometry {
                x: 2.0 * theta.cos(),
                y: 2.0 * theta.sin(),
                theta,
            };
            publisher.put(&odom, &session.now()).await.unwrap();
            tokio::time::sleep(Duration::from_millis(20)).await; // 50 Hz
        }
    });
}
