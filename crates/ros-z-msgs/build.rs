mod build_support;

use std::{env, path::PathBuf};

use color_eyre::eyre::Result;

fn main() -> Result<()> {
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);

    println!("cargo:rerun-if-changed=interfaces");

    let ros_packages = discover_ros_packages()?;

    if !ros_packages.is_empty() {
        let config = ros_z_codegen::GeneratorConfig {
            generate_cdr: true,
            generate_message_impls: true,
            output_dir: out_dir.clone(),
            external_crate: None,
            local_packages: std::collections::HashSet::new(),
            json_out: None,
        };

        let generator = ros_z_codegen::InterfaceGenerator::new(config);

        let package_refs: Vec<&std::path::Path> =
            ros_packages.iter().map(|p| p.as_path()).collect();
        generator.generate_from_interface_files(&package_refs)?;

        println!(
            "cargo:info=Generated ROS messages from {} packages",
            ros_packages.len()
        );
    }

    println!("cargo:rerun-if-changed=build.rs");

    Ok(())
}

fn discover_ros_packages() -> Result<Vec<PathBuf>> {
    let all_packages = get_all_packages();
    let asset_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("interfaces");
    let packages = build_support::discover_vendored_packages(&all_packages, &asset_root)?;

    for package_name in &all_packages {
        println!("cargo:rustc-cfg=has_{}", package_name);
    }

    Ok(packages)
}

fn get_all_packages() -> Vec<&'static str> {
    let mut names = vec!["builtin_interfaces"];

    if env::var("CARGO_FEATURE_STD_MSGS").is_ok() {
        names.push("std_msgs");
    }
    if env::var("CARGO_FEATURE_GEOMETRY_MSGS").is_ok() {
        names.push("geometry_msgs");
    }
    if env::var("CARGO_FEATURE_SENSOR_MSGS").is_ok() {
        names.push("sensor_msgs");
    }
    names
}
