mod feature_extractor;
mod odometry;
pub mod parameters;
pub mod pipeline;
mod pose_refinement;
mod triangulator;

pub use pipeline::VisualOdometryPipeline;

use std::{
    boxed::Box,
    future::Future,
    pin::Pin,
    sync::Arc,
    time::{Duration, Instant},
};

use color_eyre::Result;
use coordinate_systems::Camera;
use linear_algebra::Point3;
use nalgebra as na;

use ros_z::prelude::*;
use ros_z::qos::QosDurability;
use types::{
    stereo_camera_info::StereoCameraInfo, stereo_image_pair::StereoImagePair,
    time_wrapper::TimeWrapper,
};

use crate::parameters::StereoVisualOdometryParameters;

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("stereo_visual_odometry").build().await?;
    let node_parameters =
        node.bind_parameter_as::<StereoVisualOdometryParameters>("stereo_visual_odometry")?;
    node_parameters.add_validation_hook(StereoVisualOdometryParameters::validate)?;
    let mut parameters_receiver = node_parameters.subscribe();

    let stereo_camera_info_sub = node
        .subscriber::<StereoCameraInfo>("inputs/stereo_camera_info")?
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .build()
        .await?;

    let stereo_image_pair_sub = node
        .subscriber::<TimeWrapper<StereoImagePair>>("inputs/stereo_image_pair")?
        .build()
        .await?;

    let feature_duration_pub = node
        .publisher::<Duration>("visual_odometry/feature_extraction_duration")?
        .build()
        .await?;

    let odometry_pub = node
        .publisher::<Option<na::Isometry3<f32>>>(
            "visual_odometry/previous_left_camera_to_current_left_camera",
        )?
        .build()
        .await?;

    let odometer_pub = node
        .publisher::<na::Isometry3<f32>>("visual_odometry/current_left_camera_to_visual_odometer")?
        .build()
        .await?;

    // Caution: We don't yet differentiate between left and right camera frames.
    let triangulated_features_pub = node
        .publisher::<Vec<Point3<Camera>>>("visual_odometry/triangulated_features")?
        .build()
        .await?;

    let stereo_camera_info = stereo_camera_info_sub.recv().await?;
    let parameters = node_parameters.snapshot();
    let mut pipeline =
        VisualOdometryPipeline::new(&parameters.typed().neural_network, stereo_camera_info)?;

    loop {
        parameters_receiver
            .wait_for(|parameters| parameters.typed().enable)
            .await?;

        let stereo_image_pair = stereo_image_pair_sub.recv().await?.inner;
        let parameters = node_parameters.snapshot();
        let parameters = parameters.typed();

        let start_time = Instant::now();
        let odometry =
            pipeline.process(&stereo_image_pair, &parameters.pose_estimation_parameters)?;
        let duration = start_time.elapsed();

        odometry_pub.publish(&odometry).await?;
        odometer_pub
            .publish(&pipeline.current_left_camera_to_visual_odometer())
            .await?;
        triangulated_features_pub
            .publish(&pipeline.triangulated_features())
            .await?;
        feature_duration_pub.publish(&duration).await?;
    }
}
