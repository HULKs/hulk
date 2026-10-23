use std::sync::Arc;

use ball_filter::{BallFilter, BallHypothesis};
use color_eyre::Report;
use coordinate_systems::Pixel;
use eframe::egui::{Color32, Stroke};
use ros_z::time::Time;
use ros_z_debug::SampleRecord;
use twix_visualization::twix_painter::TwixPainter;

use super::{
    super::{
        image_overlay::ImageOverlay,
        inputs::{OverlayInput, OverlayInputs},
    },
    ball_projection::{BallProjection, aligned_window, nearest, project_covariance},
};

pub(in crate::panels::image) type BallFilterOverlay = BallFilterLayer<false>;
pub(in crate::panels::image) type BallFilterConfidenceOverlay = BallFilterLayer<true>;

#[derive(Default)]
pub(in crate::panels::image) struct BallFilterLayer<const CONFIDENCE: bool> {
    filter: Option<Arc<SampleRecord<BallFilter>>>,
    selected: Option<Option<usize>>,
    projection: BallProjection,
}

impl<const CONFIDENCE: bool> ImageOverlay for BallFilterLayer<CONFIDENCE> {
    fn inputs(&self) -> Vec<OverlayInput> {
        let mut inputs = vec![
            OverlayInput::source_time::<BallFilter>(
                "ball_filter/ball_filter_state",
                aligned_window(),
            ),
            OverlayInput::source_time::<Option<BallHypothesis>>(
                "ball_filter/best_ball_hypothesis",
                aligned_window(),
            ),
        ];
        inputs.extend(BallProjection::inputs());
        inputs
    }

    fn prepare(&mut self, time: Time, inputs: &mut OverlayInputs) -> Result<(), Report> {
        self.filter = None;
        self.selected = None;
        let filters = inputs.records(0)?;
        self.filter = nearest(&filters, time).cloned();
        let selections = inputs.records::<Option<BallHypothesis>>(1)?;
        if let Some(filter) = &self.filter
            && let Some(selected) = selections
                .iter()
                .rev()
                .find(|sample| sample.source_time == filter.source_time)
        {
            self.selected = crate::panels::ball_visualization::selected_candidate(filter, selected);
        }
        self.projection.prepare(time, inputs, 2)
    }

    fn paint(&self, painter: &TwixPainter<Pixel>) {
        let (Some(filter), Some(selected), Some(frame)) =
            (&self.filter, self.selected, self.projection.frame())
        else {
            return;
        };
        let transform = self.projection.ground_transform(frame, filter.source_time);
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
                    if let Some((pixel, covariance)) =
                        project_covariance(&frame.camera, position, frame.ball_radius, covariance)
                    {
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
        Some(match &self.filter {
            Some(sample) if sample.value.hypotheses.is_empty() => {
                "Ball Filter: no candidates in the received data".to_owned()
            }
            Some(sample) => format!("Ball Filter: {} candidates", sample.value.hypotheses.len()),
            None => "Ball Filter: waiting for candidate data".to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::panels::image::tests::raw_sample;
    use ball_filter::BallMode;
    use ros_z_debug::query::plan::State;
    use types::multivariate_normal_distribution::MultivariateNormalDistribution;

    #[test]
    fn candidates_wait_for_selection_from_the_same_filter_cycle() {
        let hypothesis = BallHypothesis {
            mode: BallMode::Resting(MultivariateNormalDistribution {
                mean: nalgebra::vector![1.0, 0.0],
                covariance: nalgebra::Matrix2::identity(),
            }),
            last_seen: Time::from_nanos(5),
            validity: 3.0,
        };
        let filter = raw_sample(
            "/filter",
            1,
            10,
            10,
            &BallFilter {
                hypotheses: vec![hypothesis.clone()],
            },
        );
        let mut inputs = OverlayInputs::default();
        let mut overlay = BallFilterOverlay::default();
        inputs.update(&[
            State::Ready(vec![filter.clone()]),
            State::Ready(vec![raw_sample(
                "/selected",
                2,
                9,
                9,
                &Some(hypothesis.clone()),
            )]),
        ]);
        overlay.prepare(Time::from_nanos(10), &mut inputs).unwrap();
        assert_eq!(overlay.selected, None);

        inputs.update(&[
            State::Ready(vec![filter.clone()]),
            State::Ready(vec![raw_sample("/selected", 3, 10, 10, &Some(hypothesis))]),
        ]);
        overlay.prepare(Time::from_nanos(10), &mut inputs).unwrap();
        assert_eq!(overlay.selected, Some(Some(0)));

        inputs.update(&[
            State::Ready(vec![filter]),
            State::Ready(vec![raw_sample(
                "/selected",
                4,
                10,
                10,
                &None::<BallHypothesis>,
            )]),
        ]);
        overlay.prepare(Time::from_nanos(10), &mut inputs).unwrap();
        assert_eq!(overlay.selected, Some(None));
        inputs.clear();
        overlay.prepare(Time::from_nanos(10), &mut inputs).unwrap();
        assert!(overlay.filter.is_none());
        assert_eq!(overlay.selected, None);
    }
}
