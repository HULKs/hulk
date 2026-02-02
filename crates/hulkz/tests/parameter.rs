//! Parameter declaration and configuration integration tests.

mod common;

use serde::{Deserialize, Serialize};

use common::{test_namespace, test_session};
use hulkz::Session;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Config {
    threshold: f64,
    count: i32,
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn local_parameter() {
    let session = test_session("param").await.unwrap();
    let node = session.create_node("test_node").build().await.unwrap();

    // Local parameter (robot-scoped) - no prefix
    let (param, driver) = node
        .declare_parameter::<f64>("max_speed")
        .build()
        .await
        .unwrap();

    let driver_handle = tokio::spawn(async move {
        let _ = driver.await;
    });

    let value = param.get().await;
    assert!((1.5 - *value).abs() < f64::EPSILON);

    driver_handle.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn global_parameter() {
    let session = test_session("param_global").await.unwrap();
    let node = session.create_node("test_node").build().await.unwrap();

    // Global parameter (fleet-wide) - "/" prefix
    let (param, driver) = node
        .declare_parameter::<String>("/fleet_id")
        .build()
        .await
        .unwrap();

    let driver_handle = tokio::spawn(async move {
        let _ = driver.await;
    });

    let value = param.get().await;
    assert_eq!(*value, "test_fleet");

    driver_handle.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn private_parameter() {
    let session = test_session("param_private").await.unwrap();
    let node = session.create_node("test_node").build().await.unwrap();

    // Private parameter (node-scoped) - "~/" prefix
    let (param, driver) = node
        .declare_parameter::<i32>("~/debug_level")
        .build()
        .await
        .unwrap();

    let driver_handle = tokio::spawn(async move {
        let _ = driver.await;
    });

    let value = param.get().await;
    assert_eq!(*value, 2);

    driver_handle.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn private_nested_parameter() {
    let session = test_session("param_nested").await.unwrap();
    let node = session.create_node("test_node").build().await.unwrap();

    // Private nested parameter
    let (param, driver) = node
        .declare_parameter::<Config>("~/config")
        .build()
        .await
        .unwrap();

    let driver_handle = tokio::spawn(async move {
        let _ = driver.await;
    });

    let value = param.get().await;
    assert!((0.1 - value.threshold).abs() < f64::EPSILON);
    assert_eq!(value.count, 42);

    driver_handle.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn parameter_with_default() {
    let session = Session::create(test_namespace("param_default")).await.unwrap();
    let node = session.create_node("test_node").build().await.unwrap();

    // Parameter not in config, but has default
    let (param, driver) = node
        .declare_parameter::<i32>("nonexistent")
        .default(42)
        .build()
        .await
        .unwrap();

    let driver_handle = tokio::spawn(async move {
        let _ = driver.await;
    });

    let value = param.get().await;
    assert_eq!(*value, 42);

    driver_handle.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn config_overrides_default() {
    let session = test_session("param_override").await.unwrap();
    let node = session.create_node("test_node").build().await.unwrap();

    // Config value should override default
    let (param, driver) = node
        .declare_parameter::<f64>("max_speed")
        .default(999.0)
        .build()
        .await
        .unwrap();

    let driver_handle = tokio::spawn(async move {
        let _ = driver.await;
    });

    // Should be config value (1.5), not default (999.0)
    let value = param.get().await;
    assert!((1.5 - *value).abs() < f64::EPSILON);

    driver_handle.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn missing_parameter_no_default_error() {
    let session = Session::create(test_namespace("param_missing")).await.unwrap();
    let node = session.create_node("test_node").build().await.unwrap();

    // No config value and no default = error
    let result = node
        .declare_parameter::<f64>("nonexistent_param")
        .build()
        .await;

    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn parameter_validation_initial() {
    let session = test_session("param_validate").await.unwrap();
    let node = session.create_node("test_node").build().await.unwrap();

    // Validation passes
    let result = node
        .declare_parameter::<f64>("max_speed")
        .validate(|v| *v > 0.0 && *v < 10.0)
        .build()
        .await;

    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn parameter_validation_fails() {
    let session = test_session("param_validate_fail").await.unwrap();
    let node = session.create_node("test_node").build().await.unwrap();

    // Validation fails (max_speed is 1.5, but we require > 100)
    let result = node
        .declare_parameter::<f64>("max_speed")
        .validate(|v| *v > 100.0)
        .build()
        .await;

    assert!(result.is_err());
}
