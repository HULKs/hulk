#[path = "../build_support.rs"]
mod build_support;

use std::path::PathBuf;

fn interface_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("interfaces")
}

#[test]
fn vendored_discovery_accepts_existing_packages_from_in_crate_interfaces() {
    let packages = build_support::discover_vendored_packages(
        &["builtin_interfaces", "sensor_msgs"],
        &interface_root(),
    )
    .unwrap();

    assert_eq!(packages.len(), 2);
    assert_eq!(packages[0], interface_root().join("builtin_interfaces"));
    assert_eq!(packages[1], interface_root().join("sensor_msgs"));
}

#[test]
fn vendored_discovery_rejects_missing_packages_instead_of_falling_back_to_system_ros() {
    let error = build_support::discover_vendored_packages(
        &["builtin_interfaces", "sensor_msgs", "missing_pkg"],
        &interface_root(),
    )
    .unwrap_err();

    assert!(error.to_string().contains("missing_pkg"));
}
