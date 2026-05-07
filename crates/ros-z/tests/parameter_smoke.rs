use std::{
    fs,
    sync::atomic::{AtomicUsize, Ordering},
};

use ros_z::prelude::*;
use serde::{Deserialize, Serialize};

static NEXT_ID: AtomicUsize = AtomicUsize::new(1);

type TestResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Debug, Clone, Serialize, Deserialize, ros_z::Message)]
#[message(name = "test_parameters::VisionParameters")]
#[serde(deny_unknown_fields)]
struct VisionParameters {
    enabled: bool,
    threshold: f64,
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn bind_merge_set_and_subscribe_work() -> TestResult {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("ros_z_parameter_smoke_{id}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root)?;

    let base = root.join("base");
    let robot = root.join("robot");
    fs::create_dir_all(&base)?;
    fs::create_dir_all(&robot)?;

    fs::write(
        base.join("ball_detector.json5"),
        r#"{
            enabled: false,
            threshold: 0.5
        }"#,
    )?;
    fs::write(
        robot.join("ball_detector.json5"),
        r#"{
            enabled: true,
            threshold: 0.8
        }"#,
    )?;

    let context = ContextBuilder::default()
        .with_mode("peer")
        .disable_multicast_scouting()
        .with_parameter_layers([base.clone(), robot.clone()])
        .build()
        .await?;
    let node = context
        .create_node("ball_detector")
        .with_namespace("vision")
        .build()
        .await?;

    let parameters = node.bind_parameter_as::<VisionParameters>("ball_detector")?;
    let snapshot = parameters.snapshot();
    assert!(snapshot.typed().enabled);
    assert_eq!(snapshot.typed().threshold, 0.8);

    let mut updates = parameters.subscribe();
    parameters.set_json(
        "threshold",
        serde_json::json!(0.9),
        robot.to_string_lossy().into_owned(),
    )?;
    updates.changed().await?;
    assert_eq!(updates.borrow().typed().threshold, 0.9);

    Ok(())
}
