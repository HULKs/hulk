//! Golden hash regression tests for retained ROS-shaped message types.
//!
//! These tests load real message definitions from the bundled native assets,
//! resolve the full type tree, and verify that the computed hashes:
//!
//! 1. Have the correct format (`RZHS01_` + 64 hex chars)
//! 2. Are deterministic across multiple calls
//! 3. Match the expected values computed from the fixed implementation
//!
//! ros-z integration tests provide end-to-end correctness validation;
//! these tests guard against regressions in future refactors.

use std::{collections::HashMap, path::PathBuf};

use ros_z_codegen::{discovery::discover_messages, resolver::Resolver, types::ResolvedMessage};

fn assets_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("interfaces")
}

fn resolve_packages(packages: &[&str]) -> HashMap<String, ResolvedMessage> {
    let assets = assets_dir();

    let mut all_messages = Vec::new();
    for pkg in packages {
        let pkg_path = assets.join(pkg);
        let msgs = discover_messages(&pkg_path, pkg)
            .unwrap_or_else(|e| panic!("Failed to discover messages in {pkg}: {e}"));
        all_messages.extend(msgs);
    }

    let mut resolver = Resolver::new();
    let resolved = resolver
        .resolve_messages(all_messages)
        .expect("Failed to resolve messages");

    resolved
        .into_iter()
        .map(|r| {
            let key = format!("{}::{}", r.parsed.package, r.parsed.name);
            (key, r)
        })
        .collect()
}

/// All packages needed to resolve the geometry/sensor message types used
/// in the tests below.
fn all_test_packages() -> &'static [&'static str] {
    &[
        "builtin_interfaces",
        "std_msgs",
        "geometry_msgs",
        "sensor_msgs",
    ]
}

fn get_hash(resolved: &HashMap<String, ResolvedMessage>, type_name: &str) -> String {
    resolved
        .get(type_name)
        .unwrap_or_else(|| panic!("Type {type_name} not found in resolved messages"))
        .schema_hash
        .to_hash_string()
}

// --- Format and determinism tests ---

#[test]
fn test_hash_format_std_msgs_string() {
    // std_msgs::Header depends on builtin_interfaces::Time, so both
    // packages are needed for the resolver to successfully resolve all messages.
    let resolved = resolve_packages(&["builtin_interfaces", "std_msgs"]);
    let hash = get_hash(&resolved, "std_msgs::String");
    assert!(
        hash.starts_with("RZHS01_"),
        "Hash should start with RZHS01_: {hash}"
    );
    assert_eq!(
        hash.len(),
        7 + 64,
        "Hash should be RZHS01_ + 64 hex chars: {hash}"
    );
}

#[test]
fn test_hash_deterministic_twist_stamped() {
    let resolved1 = resolve_packages(all_test_packages());
    let resolved2 = resolve_packages(all_test_packages());
    let hash1 = get_hash(&resolved1, "geometry_msgs::TwistStamped");
    let hash2 = get_hash(&resolved2, "geometry_msgs::TwistStamped");
    assert_eq!(hash1, hash2, "Hash must be deterministic");
}

#[test]
fn test_hashes_differ_by_type() {
    let resolved = resolve_packages(all_test_packages());
    let h_string = get_hash(&resolved, "std_msgs::String");
    let h_header = get_hash(&resolved, "std_msgs::Header");
    let h_twist_stamped = get_hash(&resolved, "geometry_msgs::TwistStamped");
    let h_pose_stamped = get_hash(&resolved, "geometry_msgs::PoseStamped");
    let h_imu = get_hash(&resolved, "sensor_msgs::Imu");

    let all = [
        &h_string,
        &h_header,
        &h_twist_stamped,
        &h_pose_stamped,
        &h_imu,
    ];
    for (i, a) in all.iter().enumerate() {
        for (j, b) in all.iter().enumerate() {
            if i != j {
                assert_ne!(
                    a, b,
                    "Hashes for different types must differ (indices {i}, {j})"
                );
            }
        }
    }
}

// --- Expected value tests ---
//
// These values were computed from the fixed implementation of `collect_referenced_types`
// (using `nested_type_name_to_key`) against the bundled native assets.
//
// End-to-end correctness for generated payloads is validated by ros-z integration tests.
// These tests guard against regressions in future refactors.

#[test]
fn test_expected_hash_std_msgs_string() {
    let resolved = resolve_packages(&["builtin_interfaces", "std_msgs"]);
    let hash = get_hash(&resolved, "std_msgs::String");
    assert_eq!(
        hash, "RZHS01_d79efd58ac7273256c5edecda25156226f6f9c66629d907336d116a5d740420a",
        "std_msgs::String hash mismatch"
    );
}

#[test]
fn test_expected_hash_std_msgs_header() {
    let resolved = resolve_packages(&["builtin_interfaces", "std_msgs"]);
    let hash = get_hash(&resolved, "std_msgs::Header");
    assert_eq!(
        hash, "RZHS01_66586dc6cb8d1911241fd4c19c330b0c89ccdc56791e64d6fd9e81dc63aac7d6",
        "std_msgs::Header hash mismatch"
    );
}

#[test]
fn test_expected_hash_twist_stamped() {
    let resolved = resolve_packages(all_test_packages());
    let hash = get_hash(&resolved, "geometry_msgs::TwistStamped");
    assert_eq!(
        hash, "RZHS01_e3714384b916438997fefc1c2083a7c8265fa6673787dd8d8f5a4145344d7aa2",
        "geometry_msgs::TwistStamped hash mismatch"
    );
}

#[test]
fn test_expected_hash_pose_stamped() {
    let resolved = resolve_packages(all_test_packages());
    let hash = get_hash(&resolved, "geometry_msgs::PoseStamped");
    assert_eq!(
        hash, "RZHS01_38119b808dd273b19f9b92cf6dd42685b7ff579782c231d04ed60525165d158d",
        "geometry_msgs::PoseStamped hash mismatch"
    );
}

#[test]
fn test_expected_hash_imu() {
    let resolved = resolve_packages(all_test_packages());
    let hash = get_hash(&resolved, "sensor_msgs::Imu");
    assert_eq!(
        hash, "RZHS01_49e7bdaef1b7e290dcb0c2efa75fef7fe178c5f26c3cbabc8be2eaf469317070",
        "sensor_msgs::Imu hash mismatch"
    );
}
