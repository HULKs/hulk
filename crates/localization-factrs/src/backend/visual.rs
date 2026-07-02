use std::time::SystemTime;

use factrs::{containers::FactorBuilder, core::Huber, traits::Optimizer};
use nalgebra::Matrix2;

use crate::{
    factors::visual_reprojection::VisualReprojectionFactor,
    measurements::VisualReprojectionMeasurement,
    symbols::{CameraIntrinsics, State},
};

use super::VinsBackend;

const GLOBAL_VISUAL_HUBER_THRESHOLD: f64 = 2.0;

impl VinsBackend {
    pub(super) fn ingest_visual(&mut self, visuals: Vec<Vec<VisualReprojectionMeasurement>>) {
        self.ingest_visual_frames(
            visuals,
            "visual",
            self.config.visual_feature_noise,
            GLOBAL_VISUAL_HUBER_THRESHOLD,
        );
    }

    pub(super) fn ingest_pose_hint_visual(
        &mut self,
        visuals: Vec<Vec<VisualReprojectionMeasurement>>,
    ) {
        self.ingest_visual_frames(
            visuals,
            "pose-hint visual",
            self.config.pose_hint_visual_feature_noise,
            self.config.pose_hint_visual_huber_threshold,
        );
    }

    fn ingest_visual_frames(
        &mut self,
        mut visuals: Vec<Vec<VisualReprojectionMeasurement>>,
        sensor_name: &str,
        noise: Matrix2<f64>,
        huber_threshold: f64,
    ) {
        visuals.retain(|visual| !visual.is_empty());
        let Some(last) = visuals.last() else {
            return;
        };

        let last_time = visual_frame_time(last);
        self.update_last_knot_time(last_time);

        let interval_groups = self.interval_groups(visuals, |visual| visual_frame_time(visual));

        for group in interval_groups {
            if !self.prepare_interval_for_measurements(group.start_index, sensor_name) {
                continue;
            }

            let keys = (
                State(group.start_index),
                State(group.start_index + 1),
                CameraIntrinsics(0),
            );
            let graph = self.optimizer.graph_mut();
            for measurement in group.measurements.into_iter().flatten() {
                let residual = VisualReprojectionFactor::new(
                    group.start_time,
                    group.end_time,
                    [measurement],
                    noise,
                );
                let factor = FactorBuilder::new(residual, keys)
                    .robust(Huber::new(huber_threshold))
                    .build();

                graph.add_factor(factor);
            }
        }
    }
}

fn visual_frame_time(visual: &[VisualReprojectionMeasurement]) -> SystemTime {
    visual
        .first()
        .expect("visual frames must contain at least one measurement")
        .time
}
