use std::sync::Arc;

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};
use linear_algebra::{Point2, Pose2, point};

use coordinate_systems::Field;
use geometry::line_segment::LineSegment;
use types::{
    field_dimensions::FieldDimensions,
    field_marks::FieldMark,
    localization::{LocalizationDebugFrame, LocalizationDebugHypothesis, MeasuredLineStatus},
};

use crate::{
    panels::map::layer::Layer, robot::Robot, twix_painter::TwixPainter, value_buffer::BufferHandle,
};

pub struct Localization {
    debug_frame: BufferHandle<Option<LocalizationDebugFrame>>,
    selected_hypothesis_index: Option<usize>,
}

impl Localization {
    pub fn set_selected_hypothesis_index(&mut self, selected_hypothesis_index: Option<usize>) {
        self.selected_hypothesis_index = selected_hypothesis_index;
    }

    pub fn pick_hypothesis_at(
        &self,
        position: Point2<Field>,
        selection_radius: f32,
    ) -> Result<Option<usize>> {
        let Some(debug_frame) = self.debug_frame.get_last_value()?.flatten() else {
            return Ok(None);
        };

        Ok(debug_frame
            .hypotheses
            .iter()
            .enumerate()
            .filter_map(|(index, hypothesis)| {
                let distance = (hypothesis_position(hypothesis) - position).norm();
                (distance <= selection_radius).then_some((index, distance))
            })
            .min_by(|(_, left_distance), (_, right_distance)| {
                left_distance.total_cmp(right_distance)
            })
            .map(|(index, _)| index))
    }
}

impl Layer<Field> for Localization {
    const NAME: &'static str = "Localization";

    fn new(robot: Arc<Robot>) -> Self {
        let debug_frame =
            robot.subscribe_value("WorldState.additional_outputs.localization.debug_frame");
        Self {
            debug_frame,
            selected_hypothesis_index: None,
        }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Field>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        let Some(debug_frame) = self.debug_frame.get_last_value()?.flatten() else {
            return Ok(());
        };

        let selected_hypothesis_index = self
            .selected_hypothesis_index
            .filter(|&index| index < debug_frame.hypotheses.len())
            .or(debug_frame.best_hypothesis_index)
            .filter(|&index| index < debug_frame.hypotheses.len());

        for (hypothesis_index, hypothesis) in debug_frame.hypotheses.iter().enumerate() {
            let pose = hypothesis_pose(hypothesis);
            let fill_color = confidence_color(hypothesis.measurement_confidence);
            let covariance_fill_color =
                Color32::from_rgba_unmultiplied(fill_color.r(), fill_color.g(), fill_color.b(), 30);
            let stroke = Stroke::new(
                if Some(hypothesis_index) == selected_hypothesis_index {
                    0.03
                } else {
                    0.02
                },
                if Some(hypothesis_index) == selected_hypothesis_index {
                    Color32::YELLOW
                } else if Some(hypothesis_index) == debug_frame.best_hypothesis_index {
                    Color32::WHITE
                } else {
                    Color32::BLACK
                },
            );

            painter.covariance(
                pose.position(),
                hypothesis.covariance.fixed_view::<2, 2>(0, 0).into_owned(),
                stroke,
                covariance_fill_color,
            );
            painter.pose(pose, 0.1, 0.16, fill_color, stroke);
        }

        if let Some(selected_hypothesis_index) = selected_hypothesis_index {
            let selected_hypothesis = &debug_frame.hypotheses[selected_hypothesis_index];
            paint_selected_hypothesis(painter, selected_hypothesis);
        }

        Ok(())
    }
}

fn paint_selected_hypothesis(
    painter: &TwixPainter<Field>,
    selected_hypothesis: &LocalizationDebugHypothesis,
) {
    for matched_line in &selected_hypothesis.final_matches {
        paint_field_mark(
            painter,
            matched_line.field_mark,
            Stroke::new(0.03, Color32::from_rgb(80, 220, 120)),
        );
    }

    for measured_line in &selected_hypothesis.candidate_summaries {
        paint_line_segment(
            painter,
            measured_line.measured_line,
            Stroke::new(0.025, Color32::from_rgb(90, 170, 255)),
        );
    }

    for matched_line in &selected_hypothesis.final_matches {
        for correspondence in [
            matched_line.correspondence_points.0,
            matched_line.correspondence_points.1,
        ] {
            painter.line_segment(
                correspondence.measured,
                correspondence.reference,
                Stroke::new(0.02, Color32::YELLOW),
            );
        }
    }

    for measured_line in selected_hypothesis
        .candidate_summaries
        .iter()
        .filter(|summary| summary.status != MeasuredLineStatus::Matched)
    {
        let color = match measured_line.status {
            MeasuredLineStatus::RejectedByThreshold => Color32::from_rgb(255, 170, 0),
            MeasuredLineStatus::NoCandidate => Color32::RED,
            MeasuredLineStatus::Matched => Color32::GREEN,
        };
        paint_line_segment(
            painter,
            measured_line.measured_line,
            Stroke::new(0.03, color),
        );
    }
}

fn confidence_color(measurement_confidence: f32) -> Color32 {
    let clamped_confidence = measurement_confidence.clamp(0.0, 1.0);
    let red = (255.0 * (1.0 - clamped_confidence)) as u8;
    let green = (255.0 * clamped_confidence) as u8;
    Color32::from_rgb(red, green, 64)
}

fn hypothesis_pose(hypothesis: &LocalizationDebugHypothesis) -> Pose2<Field> {
    Pose2::new(
        hypothesis_position(hypothesis),
        hypothesis.ground_to_field.orientation().angle(),
    )
}

fn hypothesis_position(hypothesis: &LocalizationDebugHypothesis) -> Point2<Field> {
    point![
        hypothesis.ground_to_field.translation().x(),
        hypothesis.ground_to_field.translation().y(),
    ]
}

fn paint_line_segment(
    painter: &TwixPainter<Field>,
    line_segment: LineSegment<Field>,
    stroke: Stroke,
) {
    painter.line_segment(line_segment.0, line_segment.1, stroke);
}

fn paint_field_mark(painter: &TwixPainter<Field>, field_mark: FieldMark, stroke: Stroke) {
    match field_mark {
        FieldMark::Line { line, .. } => paint_line_segment(painter, line, stroke),
        FieldMark::Circle { center, radius } => {
            painter.circle(center, radius, Color32::TRANSPARENT, stroke);
        }
    }
}
