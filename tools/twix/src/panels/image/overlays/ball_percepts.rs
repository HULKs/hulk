use color_eyre::Report;
use coordinate_systems::Pixel;
use eframe::egui::{Align2, Color32, FontId, vec2};
use ros_z::time::Time;
use twix_visualization::twix_painter::TwixPainter;
use types::{
    ball_detection::BallPercept,
    object_detection::{Object, RobocupObjectLabel},
    time_wrapper::TimeWrapper,
};

use super::super::image_overlay::{ImageOverlay, OverlayObservation};
use crate::repaint::ObservationContext;

pub(in crate::panels::image) struct BallPerceptsOverlay {
    percepts: OverlayObservation<Vec<BallPercept>>,
}

impl ImageOverlay for BallPerceptsOverlay {
    const NAME: &'static str = "Ball Percepts";
    const STORAGE_KEY: &'static str = "ball_percepts";

    fn new<C: ObservationContext>(context: &C) -> Result<Self, Report> {
        Ok(Self {
            percepts: OverlayObservation::new(context, "ball_filter/ball_percepts")?,
        })
    }

    fn paint(&self, painter: &TwixPainter<Pixel>, image_time: Time) {
        let Some(percepts) = self
            .percepts
            .nearest_source_time(image_time, std::time::Duration::ZERO)
        else {
            return;
        };
        for percept in &percepts.value {
            painter.ball(
                percept.image_location.center,
                percept.image_location.radius,
                Color32::GREEN,
            );
        }
    }
    fn latest_time(&self) -> Option<Time> {
        self.percepts.latest().map(|sample| sample.source_time)
    }
}

pub(in crate::panels::image) struct BallDetectionConfidenceOverlay {
    percepts: OverlayObservation<Vec<BallPercept>>,
    detections: OverlayObservation<TimeWrapper<Vec<Object<RobocupObjectLabel>>>>,
}

impl ImageOverlay for BallDetectionConfidenceOverlay {
    const NAME: &'static str = "Ball Percept Confidence";
    const STORAGE_KEY: &'static str = "ball_percept_confidence";

    fn new<C: ObservationContext>(context: &C) -> Result<Self, Report> {
        Ok(Self {
            detections: OverlayObservation::new(context, "detected_objects")?,
            percepts: OverlayObservation::new(context, "ball_filter/ball_percepts")?,
        })
    }

    fn paint(&self, painter: &TwixPainter<Pixel>, image_time: Time) {
        let Some(detections) = self.detections.at_time(image_time) else {
            return;
        };
        let Some(percepts) = self
            .percepts
            .nearest_source_time(image_time, std::time::Duration::ZERO)
        else {
            return;
        };
        for percept in &percepts.value {
            let Some(detection) = crate::panels::ball_visualization::detection_for_percept(
                percept,
                &detections.value.inner,
            ) else {
                continue;
            };
            let area = detection.bounding_box.area;
            let radius = (area.max.x() - area.min.x()).min(area.max.y() - area.min.y()) / 2.0;
            let screen_position = painter.transform_world_to_pixel(area.center())
                + vec2(radius * painter.scaling() + 4.0, 0.0);
            painter.floating_text(
                painter.transform_pixel_to_world(screen_position),
                Align2::LEFT_CENTER,
                format!("{:.2}", detection.bounding_box.confidence),
                FontId::proportional(12.0),
                Color32::GREEN,
            );
        }
    }

    fn latest_time(&self) -> Option<Time> {
        Some(
            self.detections
                .latest_time()?
                .min(self.percepts.latest()?.source_time),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{backend::RobotBackend, panel::PanelCreationContext};
    use eframe::egui::{CentralPanel, Context, Shape};
    use geometry::circle::Circle;
    use linear_algebra::{point, vector};
    use std::{sync::Arc, time::Duration};
    use tokio::runtime::Handle;
    use twix_visualization::twix_painter::Orientation;
    use types::multivariate_normal_distribution::MultivariateNormalDistribution;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn percepts_match_capture_time_despite_later_empty_frame() {
        let namespace = format!("/twix_percepts_{}", uuid::Uuid::new_v4().simple());
        let backend = Arc::new(
            RobotBackend::new(Handle::current(), None, namespace.clone())
                .await
                .unwrap(),
        );
        let publisher = backend
            .node()
            .publisher::<Vec<BallPercept>>(&format!("{namespace}/ball_filter/ball_percepts"))
            .build()
            .await
            .unwrap();
        let context = PanelCreationContext {
            backend,
            value: None,
            egui_context: Context::default(),
        };
        let overlay = BallPerceptsOverlay::new(&context).unwrap();
        let frame = Time::from_nanos(1_000_000_000);
        let next_frame = frame + Duration::from_millis(33);
        let percept = BallPercept {
            image_location: Circle::new(point![32.0, 32.0], 4.0),
            percept_in_ground: MultivariateNormalDistribution {
                mean: nalgebra::vector![1.0, 0.0],
                covariance: nalgebra::Matrix2::identity(),
            },
        };
        tokio::time::timeout(Duration::from_secs(5), async {
            while overlay
                .percepts
                .nearest_source_time(frame, Duration::ZERO)
                .is_none()
                || overlay
                    .percepts
                    .nearest_source_time(next_frame, Duration::ZERO)
                    .is_none()
            {
                publisher
                    .publish_with_source_time(&vec![percept], frame)
                    .await
                    .unwrap();
                publisher
                    .publish_with_source_time(&vec![], next_frame)
                    .await
                    .unwrap();
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        let ball_colors = |time| {
            let output = context.egui_context.run_ui(Default::default(), |ui| {
                CentralPanel::default().show(ui, |ui| {
                    let (_, painter) = TwixPainter::allocate(
                        ui,
                        vector![64.0, 64.0],
                        point![0.0, 0.0],
                        Orientation::LeftHanded,
                    );
                    overlay.paint(&painter, time);
                });
            });
            output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    Shape::Circle(circle) => Some(circle.fill),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(ball_colors(frame), [Color32::GREEN]);
        assert!(ball_colors(next_frame).is_empty());
        assert!(ball_colors(frame + Duration::from_millis(1)).is_empty());
        assert_eq!(overlay.latest_time(), Some(next_frame));
    }
}
