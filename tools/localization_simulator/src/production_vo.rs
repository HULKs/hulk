use std::{sync::Arc, time::Instant};

use color_eyre::{Result, eyre::Context as _};
use nalgebra::Isometry3;
use ros_z::time::Time;
use ros2::{
    sensor_msgs::{camera_info::CameraInfo, image::Image},
    std_msgs::header::Header,
};
use serde::Serialize;
use stereo_visual_odometry::{
    OdometryDiagnostics, PoseEvaluationDiagnostics, TrackingOutcome, VisualOdometryPipeline,
    parameters::StereoVisualOdometryParameters,
};
use types::{stereo_camera_info::StereoCameraInfo, stereo_image_pair::StereoImagePair};

use crate::{
    sensors::VisualOdometryMeasurement,
    stereo_render::{BASELINE, CX, CY, FX, FY, HEIGHT, RenderedStereoRgba, StereoRenderer, WIDTH},
};

#[derive(Clone, Copy, Debug, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductionVoStatus {
    #[default]
    Initializing,
    Estimated,
    Reset,
}

#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct ProductionVoDiagnostics {
    pub status: ProductionVoStatus,
    pub processing_seconds: f64,
    pub correspondences: usize,
    pub inliers: usize,
    pub triangulated: usize,
    pub used_identity_initialization: bool,
    pub previous_disparity_mean: Option<f32>,
    pub previous_disparity_below_4px: usize,
    pub previous_disparity_below_6px: usize,
    pub previous_disparity_below_8px: usize,
    pub estimated_left_rmse: Option<f32>,
    pub estimated_right_rmse: Option<f32>,
    pub truth_left_rmse: Option<f32>,
    pub truth_right_rmse: Option<f32>,
    pub truth_left_inliers_1px: usize,
    pub truth_left_inliers_2px: usize,
    pub truth_left_inliers_6px: usize,
}

pub(crate) struct ProductionVisualOdometry {
    renderer: StereoRenderer,
    pipeline: VisualOdometryPipeline,
    parameters: StereoVisualOdometryParameters,
    previous_left_camera_to_field: Option<Isometry3<f32>>,
    frame_identifier: u32,
}

impl ProductionVisualOdometry {
    pub(crate) fn new() -> Result<Self> {
        let parameters: StereoVisualOdometryParameters = json5::from_str(include_str!(
            "../../../etc/parameters/base/stereo_visual_odometry.json5"
        ))
        .wrap_err("failed to parse stereo visual odometry parameters")?;
        parameters.validate().map_err(color_eyre::Report::msg)?;
        let pipeline =
            VisualOdometryPipeline::new(&parameters.neural_network, stereo_camera_info())?;
        Ok(Self {
            renderer: StereoRenderer::new(),
            pipeline,
            parameters,
            previous_left_camera_to_field: None,
            frame_identifier: 0,
        })
    }

    pub(crate) fn measure(
        &mut self,
        time: Time,
        right_reported_time: Time,
        left_camera_to_field: &Isometry3<f32>,
        right_camera_to_field: &Isometry3<f32>,
    ) -> VisualOdometryMeasurement {
        let rendered = self
            .renderer
            .render(left_camera_to_field, right_camera_to_field);
        let pair = stereo_pair(self.frame_identifier, time, right_reported_time, rendered);
        self.frame_identifier = self.frame_identifier.wrapping_add(1);

        let started = Instant::now();
        let output = self
            .pipeline
            .process(&pair, &self.parameters.pose_estimation_parameters);
        let processing_seconds = started.elapsed().as_secs_f64();
        let odometry_diagnostics = self.pipeline.latest_odometry_diagnostics();
        let triangulated = self.pipeline.triangulated_feature_count();
        let estimated_evaluation = output
            .previous_to_current()
            .as_ref()
            .and_then(|pose| self.pipeline.evaluate_previous_to_current(pose));
        let truth_evaluation = self.previous_left_camera_to_field.and_then(|previous| {
            self.pipeline
                .evaluate_previous_to_current(&(left_camera_to_field.inverse() * previous))
        });

        let status = match output.outcome {
            TrackingOutcome::Estimated => ProductionVoStatus::Estimated,
            TrackingOutcome::Initializing => ProductionVoStatus::Initializing,
            TrackingOutcome::Reset { .. } => ProductionVoStatus::Reset,
        };
        self.previous_left_camera_to_field =
            (!matches!(status, ProductionVoStatus::Reset)).then_some(*left_camera_to_field);

        VisualOdometryMeasurement {
            delta: output.odometer.delta.clone(),
            odometer: output.odometer,
            production_diagnostics: Some(diagnostics(
                status,
                processing_seconds,
                odometry_diagnostics,
                triangulated,
                estimated_evaluation,
                truth_evaluation,
            )),
        }
    }
}

fn diagnostics(
    status: ProductionVoStatus,
    processing_seconds: f64,
    diagnostics: OdometryDiagnostics,
    triangulated: usize,
    estimated: Option<PoseEvaluationDiagnostics>,
    truth: Option<PoseEvaluationDiagnostics>,
) -> ProductionVoDiagnostics {
    ProductionVoDiagnostics {
        status,
        processing_seconds,
        correspondences: diagnostics.correspondences,
        inliers: diagnostics.left_ransac_inliers,
        triangulated,
        used_identity_initialization: diagnostics.used_identity_initialization,
        previous_disparity_mean: diagnostics.previous_disparity_mean,
        previous_disparity_below_4px: diagnostics.previous_disparity_below_4px,
        previous_disparity_below_6px: diagnostics.previous_disparity_below_6px,
        previous_disparity_below_8px: diagnostics.previous_disparity_below_8px,
        estimated_left_rmse: estimated.and_then(|evaluation| evaluation.left_rmse),
        estimated_right_rmse: estimated.and_then(|evaluation| evaluation.right_rmse),
        truth_left_rmse: truth.and_then(|evaluation| evaluation.left_rmse),
        truth_right_rmse: truth.and_then(|evaluation| evaluation.right_rmse),
        truth_left_inliers_1px: truth.map_or(0, |evaluation| evaluation.left_inliers_1px),
        truth_left_inliers_2px: truth.map_or(0, |evaluation| evaluation.left_inliers_2px),
        truth_left_inliers_6px: truth.map_or(0, |evaluation| evaluation.left_inliers_6px),
    }
}

fn stereo_pair(
    frame_identifier: u32,
    left_time: Time,
    right_time: Time,
    rendered: RenderedStereoRgba,
) -> StereoImagePair {
    StereoImagePair {
        frame_identifier,
        left: image(left_time, "left_camera", rendered.left),
        right: image(right_time, "right_camera", rendered.right),
    }
}

fn image(time: Time, frame_id: &str, rgba: Vec<u8>) -> Image {
    Image {
        header: Header {
            stamp: time.to_wallclock().into(),
            frame_id: frame_id.to_string(),
        },
        height: HEIGHT,
        width: WIDTH,
        encoding: "nv12".to_string(),
        is_bigendian: 0,
        step: WIDTH,
        data: Arc::from(rgba_to_nv12(&rgba).into_boxed_slice()),
    }
}

fn rgba_to_nv12(rgba: &[u8]) -> Vec<u8> {
    assert_eq!(rgba.len(), (WIDTH * HEIGHT * 4) as usize);
    let pixel_count = (WIDTH * HEIGHT) as usize;
    let mut nv12 = vec![128; pixel_count * 3 / 2];
    for (index, pixel) in rgba.as_chunks::<4>().0.iter().enumerate() {
        nv12[index] = luma(pixel[0], pixel[1], pixel[2]);
    }
    for y in (0..HEIGHT as usize).step_by(2) {
        for x in (0..WIDTH as usize).step_by(2) {
            let mut red = 0.0;
            let mut green = 0.0;
            let mut blue = 0.0;
            for dy in 0..2 {
                for dx in 0..2 {
                    let index = ((y + dy) * WIDTH as usize + x + dx) * 4;
                    red += rgba[index] as f32;
                    green += rgba[index + 1] as f32;
                    blue += rgba[index + 2] as f32;
                }
            }
            red *= 0.25;
            green *= 0.25;
            blue *= 0.25;
            let chroma = pixel_count + y / 2 * WIDTH as usize + x;
            nv12[chroma] = (-0.169 * red - 0.331 * green + 0.5 * blue + 128.0)
                .round()
                .clamp(0.0, 255.0) as u8;
            nv12[chroma + 1] = (0.5 * red - 0.419 * green - 0.081 * blue + 128.0)
                .round()
                .clamp(0.0, 255.0) as u8;
        }
    }
    nv12
}

fn luma(red: u8, green: u8, blue: u8) -> u8 {
    (0.299 * red as f32 + 0.587 * green as f32 + 0.114 * blue as f32)
        .round()
        .clamp(0.0, 255.0) as u8
}

fn stereo_camera_info() -> StereoCameraInfo {
    StereoCameraInfo {
        left: camera_info(0.0),
        right: camera_info(-(FX * BASELINE) as f64),
    }
}

fn camera_info(tx: f64) -> CameraInfo {
    CameraInfo {
        height: HEIGHT,
        width: WIDTH,
        distortion_model: "plumb_bob".to_string(),
        k: [
            FX as f64, 0.0, CX as f64, 0.0, FY as f64, CY as f64, 0.0, 0.0, 1.0,
        ],
        r: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
        p: [
            FX as f64, 0.0, CX as f64, tx, 0.0, FY as f64, CY as f64, 0.0, 0.0, 0.0, 1.0, 0.0,
        ],
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgba_conversion_produces_tightly_packed_nv12() {
        let rgba = vec![64; (WIDTH * HEIGHT * 4) as usize];
        let nv12 = rgba_to_nv12(&rgba);
        assert_eq!(nv12.len(), (WIDTH * HEIGHT * 3 / 2) as usize);
        assert!(
            nv12[..(WIDTH * HEIGHT) as usize]
                .iter()
                .all(|value| *value == 64)
        );
    }

    #[test]
    fn stereo_calibration_has_positive_baseline() {
        let info = stereo_camera_info();
        assert_eq!(info.left.p[3], 0.0);
        assert!(info.right.p[3] < 0.0);
        assert!((info.right.p[3] / -(FX as f64) - BASELINE as f64).abs() < 1.0e-6);
    }
}
