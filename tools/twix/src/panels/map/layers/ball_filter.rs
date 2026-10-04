use crate::repaint::{ObservationContext, ObservationRepaint, RepaintOnUpdates};
use std::sync::Arc;

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use ball_filter::{BallFilter as BallFiltering, BallHypothesis};
use coordinate_systems::Ground;
use ros_z_debug::{RetentionPolicy, TopicObservation};
use types::field_dimensions::FieldDimensions;

use crate::{backend::RobotBackend, panels::map::layer::Layer};
use twix_visualization::twix_painter::TwixPainter;

pub type BallFilter = BallFilterLayer<false>;
pub type BallFilterConfidence = BallFilterLayer<true>;

pub struct BallFilterLayer<const CONFIDENCE: bool> {
    backend: Arc<RobotBackend>,
    filter: TopicObservation<BallFiltering>,
    selected: TopicObservation<Option<BallHypothesis>>,
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use ball_filter::BallMode;
    use eframe::egui::{CentralPanel, Context, Shape};
    use linear_algebra::{point, vector};
    use ros_z::time::Time;
    use tokio::runtime::Handle;
    use twix_visualization::twix_painter::Orientation;
    use types::multivariate_normal_distribution::MultivariateNormalDistribution;

    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn reports_candidate_publisher_message_mismatch() {
        let namespace = format!("/twix_mismatch_{}", uuid::Uuid::new_v4().simple());
        let backend = Arc::new(
            RobotBackend::new(Handle::current(), None, namespace.clone())
                .await
                .unwrap(),
        );
        // A publisher can be visible on the same topic yet never match the
        // typed subscription because its message schema differs.
        let _publisher = backend
            .node()
            .publisher::<Vec<BallHypothesis>>(&format!("{namespace}/ball_filter/ball_filter_state"))
            .build()
            .await
            .unwrap();
        let layer = BallFilter::new(backend.clone());
        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                if layer
                    .status()
                    .unwrap()
                    .contains("incompatible message format")
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("incompatible publishers must not be reported as merely waiting for data");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn candidates_option_draws_only_unselected_candidates() {
        let namespace = format!("/twix_candidates_{}", uuid::Uuid::new_v4().simple());
        let backend = Arc::new(
            RobotBackend::new(Handle::current(), None, namespace.clone())
                .await
                .unwrap(),
        );
        let publisher = backend
            .node()
            .publisher::<BallFiltering>(&format!("{namespace}/ball_filter/ball_filter_state"))
            .build()
            .await
            .unwrap();
        let selected_publisher = backend
            .node()
            .publisher::<Option<BallHypothesis>>(&format!(
                "{namespace}/ball_filter/best_ball_hypothesis"
            ))
            .build()
            .await
            .unwrap();
        let layer = BallFilter::new(backend.clone());
        let selected = BallHypothesis {
            mode: BallMode::Resting(MultivariateNormalDistribution {
                mean: nalgebra::vector![0.5, 0.5],
                covariance: nalgebra::Matrix2::identity() * 0.01,
            }),
            last_seen: Time::zero(),
            validity: 3.0,
        };
        let candidate = BallHypothesis {
            mode: BallMode::Moving(MultivariateNormalDistribution {
                mean: nalgebra::vector![-0.5, -0.5, 0.0, 0.0],
                covariance: nalgebra::Matrix4::identity() * 0.1,
            }),
            last_seen: Time::zero(),
            validity: 0.3,
        };
        let filter = BallFiltering {
            hypotheses: vec![candidate, selected.clone()],
        };
        tokio::time::timeout(Duration::from_secs(5), async {
            while layer.filter.latest().is_none() || layer.selected.latest().is_none() {
                publisher
                    .publish_with_source_time(&filter, Time::zero())
                    .await
                    .unwrap();
                selected_publisher
                    .publish_with_source_time(&Some(selected.clone()), Time::zero())
                    .await
                    .unwrap();
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("filter candidates and selection should reach the overlay");

        let rendered_balls = || {
            let output = Context::default().run_ui(Default::default(), |context| {
                CentralPanel::default().show(context, |ui| {
                    let (_, painter) = TwixPainter::allocate(
                        ui,
                        vector![2.0, 2.0],
                        point![1.0, -1.0],
                        Orientation::RightHanded,
                    );
                    layer.paint(&painter, &FieldDimensions::default()).unwrap();
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
        assert_eq!(rendered_balls(), [Color32::GRAY]);

        // A new state arrives before its selection. Even an unchanged selected
        // ball must not allow the old cycle to classify the new candidates.
        let next_time = Time::from_nanos(10_000_000);
        publisher
            .publish_with_source_time(&filter, next_time)
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_secs(3), async {
            while layer.filter.latest().unwrap().source_time != next_time {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        assert!(rendered_balls().is_empty());

        // No hypothesis is selected in this complete cycle: draw both gray.
        selected_publisher
            .publish_with_source_time(&None, next_time)
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_secs(3), async {
            while layer.selected.latest().unwrap().source_time != next_time {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(rendered_balls(), [Color32::GRAY, Color32::GRAY]);
    }
}

impl<const CONFIDENCE: bool> Layer<Ground> for BallFilterLayer<CONFIDENCE> {
    const NAME: &'static str = if CONFIDENCE {
        "Ball Filter Confidence"
    } else {
        "Ball Filter Candidates"
    };
    const STORAGE_KEY: Option<&'static str> = Some(if CONFIDENCE {
        "ball_filter_confidence"
    } else {
        "ball_filter_candidates"
    });

    fn new(backend: Arc<RobotBackend>) -> Self {
        let _runtime_handle = backend.runtime_handle().enter();

        let filter = backend
            .observer()
            .observe_typed("ball_filter/ball_filter_state")
            .expect("failed to construct ball filter state observer")
            .spawn();
        let selected = backend
            .observer()
            .observe_typed("ball_filter/best_ball_hypothesis")
            .expect("failed to construct best ball hypothesis observer")
            .retention(RetentionPolicy::time_window(std::time::Duration::from_secs(2)).unwrap())
            .spawn();

        Self {
            backend: backend.clone(),
            filter,
            selected,
        }
    }

    fn repaint_on_updates(&self, context: &impl ObservationContext) -> Vec<ObservationRepaint> {
        vec![
            self.filter.repaint_on_updates(context),
            self.selected.repaint_on_updates(context),
        ]
    }

    fn paint(
        &self,
        painter: &TwixPainter<Ground>,
        field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        let filter_sample = self.filter.latest();
        let Some(filter_sample) = filter_sample.as_deref() else {
            return Ok(());
        };
        let Some(selected_sample) = self.selected.get_nearest(filter_sample.source_time) else {
            return Ok(());
        };
        let Some(selected) =
            crate::panels::ball_visualization::selected_candidate(filter_sample, &selected_sample)
        else {
            return Ok(());
        };

        // Draw selected uncertainty last; the selected icon has its own option.
        for selected_pass in [false, true] {
            if selected_pass && !CONFIDENCE {
                continue;
            }
            for (index, hypothesis) in filter_sample.value.hypotheses.iter().enumerate() {
                if (selected == Some(index)) != selected_pass {
                    continue;
                }
                let color = if selected_pass {
                    Color32::BLUE
                } else {
                    Color32::GRAY
                };
                let ball = hypothesis.position();
                if CONFIDENCE {
                    painter.covariance(
                        ball.position,
                        hypothesis.position_covariance(),
                        Stroke::new(1.5 / painter.scaling(), color),
                        Color32::TRANSPARENT,
                    );
                } else {
                    painter.line_segment(
                        ball.position,
                        ball.position + ball.velocity,
                        Stroke::new(0.01, color),
                    );
                    painter.ball(ball.position, field_dimensions.ball_radius, color);
                }
            }
        }

        Ok(())
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
