use std::path::Path;

use crate::{
    feature_extractor::{FeatureExtractor, NUM_KEYPOINTS, PreviousFeatureState},
    odometry::{OdometryDiagnostics, OdometryScratch, PreviousFrame, estimate_previous_to_current},
    parameters::StereoVisualOdometryPoseEstimationParameters,
    tracking::{FrameOutput, TrackingOutcome, TrackingState},
    triangulator::StereoTriangulator,
};

use coordinate_systems::Camera;
use linear_algebra::{Point3, point};
use types::{stereo_camera_info::StereoCameraInfo, stereo_image_pair::StereoImagePair};

use color_eyre::{Result, eyre::Report};
use nalgebra as na;

/// Stateful stereo visual odometry pipeline.
///
/// Owns feature history and applies the shared continuity policy after each pair
/// estimate. Callers consume explicit outcomes and never reset feature history.
pub struct VisualOdometryPipeline {
    feature_extractor: FeatureExtractor,
    triangulator: StereoTriangulator,
    previous_features: PreviousFeatureState,
    previous_frame: Option<PreviousFrame>,
    current_points: Vec<crate::triangulator::StereoPoint>,
    tracking: TrackingState,
    odometry_scratch: OdometryScratch,
}

impl VisualOdometryPipeline {
    /// Create a pipeline for one fixed stereo camera calibration and ONNX model.
    pub fn new(model_path: impl AsRef<Path>, stereo_camera_info: StereoCameraInfo) -> Result<Self> {
        Ok(Self {
            feature_extractor: FeatureExtractor::new(model_path)?,
            triangulator: StereoTriangulator::new(
                &stereo_camera_info.left,
                &stereo_camera_info.right,
            )?,
            previous_features: PreviousFeatureState::new(),
            previous_frame: None,
            current_points: Vec::with_capacity(NUM_KEYPOINTS),
            tracking: TrackingState::default(),
            odometry_scratch: OdometryScratch::new(),
        })
    }

    /// Process one NV12 stereo frame pair.
    ///
    /// Failures start a new epoch and discard the failed frame as a reference.
    /// The next successfully extracted frame initializes the new sequence.
    pub fn process(
        &mut self,
        stereo_image_pair: &StereoImagePair,
        parameters: &StereoVisualOdometryPoseEstimationParameters,
    ) -> FrameOutput {
        self.odometry_scratch.reset_diagnostics();
        let estimate = self.estimate_pair(stereo_image_pair, parameters);
        let output = self
            .tracking
            .finish(stereo_image_pair.left.header.stamp.into(), estimate);
        if matches!(output.outcome, TrackingOutcome::Reset { .. }) {
            self.previous_features = PreviousFeatureState::new();
            self.previous_frame = None;
        }
        output
    }

    /// Numerical pair estimation and feature-buffer management only.
    fn estimate_pair(
        &mut self,
        stereo_image_pair: &StereoImagePair,
        parameters: &StereoVisualOdometryPoseEstimationParameters,
    ) -> Result<Option<na::Isometry3<f32>>> {
        parameters.validate().map_err(Report::msg)?;

        let odometry = {
            let features = self
                .feature_extractor
                .extract(stereo_image_pair, &self.previous_features)?;
            let current_left = features.current_left()?;
            let current_right = features.current_right()?;
            let stereo_matches = features.stereo_matches()?;

            self.triangulator.triangulate_into(
                current_left,
                current_right,
                stereo_matches,
                parameters.max_vertical_disparity_px,
                &mut self.current_points,
            );

            let odometry = if let Some(previous_frame) = self.previous_frame.as_ref() {
                let temporal_matches = features.temporal_matches()?;
                estimate_previous_to_current(
                    previous_frame,
                    &current_left,
                    &self.current_points,
                    &temporal_matches,
                    &self.triangulator,
                    parameters,
                    &mut self.odometry_scratch,
                )
            } else {
                None
            };
            features.copy_current_left_to(&mut self.previous_features)?;
            odometry
        };

        if let Some(previous_frame) = self.previous_frame.as_mut() {
            previous_frame.replace_stereo_points(&self.current_points);
        } else {
            self.previous_frame = Some(PreviousFrame::from_stereo_points(&self.current_points));
        }

        Ok(odometry)
    }

    pub fn latest_odometry_diagnostics(&self) -> OdometryDiagnostics {
        self.odometry_scratch.diagnostics()
    }

    /// Return the stereo points triangulated from the most recently processed frame.
    ///
    /// Points are expressed in the current left-camera frame.
    pub fn triangulated_features(&self) -> Vec<Point3<Camera>> {
        self.current_points
            .iter()
            .map(|point| {
                point! {
                    point.position.x,
                    point.position.y,
                    point.position.z,
                }
            })
            .collect()
    }
}
