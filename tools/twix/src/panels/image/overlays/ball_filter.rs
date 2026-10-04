use std::sync::Arc;

use ball_filter::{BallFilter, BallHypothesis};
use color_eyre::Report;
use coordinate_systems::Pixel;
use eframe::egui::{Color32, Stroke};
use ros_z::time::Time;
use twix_visualization::twix_painter::TwixPainter;

use super::{
    super::image_overlay::{ImageOverlay, OverlayObservation},
    ball_projection::{ALIGNMENT_TOLERANCE, BallProjection, project_covariance},
};
use crate::{backend::RobotBackend, repaint::ObservationContext};

pub(in crate::panels::image) type BallFilterOverlay = BallFilterLayer<false>;
pub(in crate::panels::image) type BallFilterConfidenceOverlay = BallFilterLayer<true>;

pub(in crate::panels::image) struct BallFilterLayer<const CONFIDENCE: bool> {
    backend: Arc<RobotBackend>,
    filter: OverlayObservation<BallFilter>,
    selected: OverlayObservation<Option<BallHypothesis>>,
    projection: BallProjection,
}

impl<const CONFIDENCE: bool> ImageOverlay for BallFilterLayer<CONFIDENCE> {
    const NAME: &'static str = if CONFIDENCE {
        "Ball Filter Confidence"
    } else {
        "Ball Filter Candidates"
    };
    const STORAGE_KEY: &'static str = if CONFIDENCE {
        "ball_filter_confidence"
    } else {
        "ball_filter_candidates"
    };

    fn new<C: ObservationContext>(context: &C) -> Result<Self, Report> {
        Ok(Self {
            backend: context.backend().clone(),
            filter: OverlayObservation::new(context, "ball_filter/ball_filter_state")?,
            selected: OverlayObservation::new(context, "ball_filter/best_ball_hypothesis")?,
            projection: BallProjection::new(context)?,
        })
    }

    fn paint(&self, painter: &TwixPainter<Pixel>, image_time: Time) {
        let Some(filter) = self
            .filter
            .nearest_source_time(image_time, ALIGNMENT_TOLERANCE)
        else {
            return;
        };
        let Some(frame) = self.projection.at_time(image_time) else {
            return;
        };
        let Some(selected_sample) = self
            .selected
            .nearest_source_time(filter.source_time, std::time::Duration::ZERO)
        else {
            return;
        };
        let Some(selected) =
            crate::panels::ball_visualization::selected_candidate(&filter, &selected_sample)
        else {
            return;
        };
        let transform = self.projection.ground_transform(&frame, filter.source_time);
        let rotation = transform.inner.rotation.to_rotation_matrix();
        for selected_pass in [false, true] {
            if selected_pass && !CONFIDENCE {
                continue;
            }
            for (index, hypothesis) in filter.value.hypotheses.iter().enumerate() {
                let is_selected = selected == Some(index);
                if is_selected != selected_pass {
                    continue;
                }
                let color = if is_selected {
                    Color32::BLUE
                } else {
                    Color32::GRAY
                };
                let position = transform * hypothesis.position().position;
                if CONFIDENCE {
                    let covariance = rotation.matrix()
                        * hypothesis.position_covariance()
                        * rotation.matrix().transpose();
                    if let Some((pixel, covariance)) = project_covariance(
                        &frame.camera.value.inner,
                        position,
                        frame.ball_radius,
                        covariance,
                    ) {
                        painter.covariance(
                            pixel,
                            covariance,
                            Stroke::new(1.5 / painter.scaling(), color),
                            Color32::TRANSPARENT,
                        );
                    }
                } else {
                    frame.paint_ground(painter, position, color);
                }
            }
        }
    }

    fn status(&self) -> Option<String> {
        Some(crate::panels::ball_visualization::ball_filter_status(
            &self.backend,
            self.filter
                .latest()
                .map(|sample| sample.value.hypotheses.len()),
            self.filter.status(),
        ))
    }
}
