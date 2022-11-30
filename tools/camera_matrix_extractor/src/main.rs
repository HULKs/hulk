mod camera_matrix;
mod forward_kinematics;
mod ground_calculation;
mod inertial_measurement_unit;
mod joints;
mod replay_frame;
mod robot_dimensions;
mod robot_kinematics;
mod support_foot;

use std::path::PathBuf;

use anyhow::Context;
use nalgebra::vector;
use serde_json::to_value;
use structopt::StructOpt;

use crate::{
    camera_matrix::{CameraMatrix, CameraPosition},
    ground_calculation::Ground,
    inertial_measurement_unit::InertialMeasurementUnitData,
    joints::Joints,
    replay_frame::to_replay_frame,
    robot_kinematics::RobotKinematics,
    support_foot::SupportFoot,
};

#[derive(Debug, StructOpt)]
#[structopt(
    name = "camera_matrix_extractor",
    about = "Small utility to extract camera matrixes for Rust from old replay.json"
)]
struct Arguments {
    /// Path to replay.json
    path_to_replay_json: PathBuf,

    /// Image filename prefix, e.g. "topImage_123456"
    image_prefix: String,
}

fn main() -> anyhow::Result<()> {
    let arguments = Arguments::from_args();

    let replay_frame = to_replay_frame(arguments.path_to_replay_json, &arguments.image_prefix)
        .context("to_replay_frame(\"replay.json\")")?;
    let support_foot =
        SupportFoot::try_from(&replay_frame).context("SupportFoot::try_from(replay_frame)")?;
    let inertial_measurement_unit = InertialMeasurementUnitData::try_from(&replay_frame)
        .context("InertialMeasurementUnitData::try_from(replay_frame)")?;
    let joints = Joints::try_from(&replay_frame).context("Joints::try_from(replay_frame)")?;
    let robot_kinematics = RobotKinematics::from(joints);
    let ground = Ground::from((
        inertial_measurement_unit,
        robot_kinematics.clone(),
        support_foot,
    ));
    let camera_position = if arguments.image_prefix.starts_with("top") {
        CameraPosition::Top
    } else {
        CameraPosition::Bottom
    };
    let extrinsic_rotation = if arguments.image_prefix.starts_with("top") {
        vector![0.5, -5.5, 1.0]
    } else {
        vector![0.8, -4.0, 1.0]
    };
    let focal_length = vector![0.95, 1.27];
    let optical_center = vector![0.5, 0.5];
    let camera_matrix = CameraMatrix::from_ground_and_robot_kinematics(
        camera_position,
        ground,
        robot_kinematics,
        extrinsic_rotation,
        focal_length,
        optical_center,
    );
    println!(
        "{:#}",
        to_value(camera_matrix).context("to_value(&camera_matrix)")?
    );
    Ok(())
}
