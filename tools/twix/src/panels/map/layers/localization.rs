use std::sync::Arc;

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};
use linear_algebra::{Point2, Pose2, point};

use coordinate_systems::Field;
use geometry::line_segment::LineSegment;
use types::{
    field_dimensions::FieldDimensions,
    field_marks::FieldMark,
    localization::{
        GoalPostPairAssociationDebug, LineAssociationDebug, LocalizationDebugFrame,
        LocalizationDebugHypothesis, PointAssociationDebug,
    },
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
            let fill_color = confidence_color(hypothesis.score);
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
    for line_association in &selected_hypothesis.line_associations {
        paint_field_mark(
            painter,
            line_association.matched_field_mark,
            Stroke::new(0.03, Color32::from_rgb(80, 220, 120)),
        );
        paint_line_segment(
            painter,
            line_association.measured_line,
            Stroke::new(0.025, Color32::from_rgb(90, 170, 255)),
        );
        paint_correspondence_segments(painter, line_association);
    }

    for &unmatched_line in &selected_hypothesis.unmatched_lines_in_field {
        paint_line_segment(painter, unmatched_line, Stroke::new(0.03, Color32::RED));
    }

    for association in &selected_hypothesis.penalty_spot_associations {
        paint_point_association(
            painter,
            association,
            Color32::from_rgb(80, 220, 120),
            Color32::from_rgb(70, 160, 255),
            Color32::YELLOW,
        );
    }
    for &unmatched_point in &selected_hypothesis.unmatched_penalty_spots_in_field {
        paint_point(
            painter,
            unmatched_point,
            0.06,
            Color32::from_rgb(255, 140, 0),
        );
    }

    for association in &selected_hypothesis.single_goal_post_associations {
        paint_point_association(
            painter,
            association,
            Color32::from_rgb(60, 220, 140),
            Color32::from_rgb(70, 160, 255),
            Color32::YELLOW,
        );
    }
    for &unmatched_point in &selected_hypothesis.unmatched_goal_posts_in_field {
        paint_point(painter, unmatched_point, 0.07, Color32::RED);
    }

    if let Some(goal_post_pair_association) = &selected_hypothesis.goal_post_pair_association {
        paint_goal_post_pair_association(painter, goal_post_pair_association);
    }
}

fn paint_correspondence_segments(
    painter: &TwixPainter<Field>,
    line_association: &LineAssociationDebug,
) {
    for correspondence in [
        line_association.correspondence_points.0,
        line_association.correspondence_points.1,
    ] {
        painter.line_segment(
            correspondence.measured,
            correspondence.reference,
            Stroke::new(0.02, Color32::YELLOW),
        );
    }
}

fn paint_point_association(
    painter: &TwixPainter<Field>,
    association: &PointAssociationDebug,
    reference_color: Color32,
    measured_color: Color32,
    association_color: Color32,
) {
    paint_point(
        painter,
        association.measured_point_in_field,
        0.06,
        measured_color,
    );
    if let Some(reference_point) = association.matched_reference_point {
        paint_point(painter, reference_point, 0.06, reference_color);
        painter.line_segment(
            association.measured_point_in_field,
            reference_point,
            Stroke::new(
                0.02,
                if association.accepted {
                    association_color
                } else {
                    Color32::from_rgb(255, 140, 0)
                },
            ),
        );
    }
}

fn paint_goal_post_pair_association(
    painter: &TwixPainter<Field>,
    pair_association: &GoalPostPairAssociationDebug,
) {
    paint_point(
        painter,
        pair_association.measured_posts_in_field.0,
        0.07,
        Color32::from_rgb(70, 160, 255),
    );
    paint_point(
        painter,
        pair_association.measured_posts_in_field.1,
        0.07,
        Color32::from_rgb(70, 160, 255),
    );
    paint_point(
        painter,
        pair_association.matched_reference_posts.0,
        0.07,
        Color32::from_rgb(60, 220, 140),
    );
    paint_point(
        painter,
        pair_association.matched_reference_posts.1,
        0.07,
        Color32::from_rgb(60, 220, 140),
    );

    painter.line_segment(
        pair_association.measured_posts_in_field.0,
        pair_association.matched_reference_posts.0,
        Stroke::new(
            0.02,
            if pair_association.accepted {
                Color32::YELLOW
            } else {
                Color32::from_rgb(255, 140, 0)
            },
        ),
    );
    painter.line_segment(
        pair_association.measured_posts_in_field.1,
        pair_association.matched_reference_posts.1,
        Stroke::new(
            0.02,
            if pair_association.accepted {
                Color32::YELLOW
            } else {
                Color32::from_rgb(255, 140, 0)
            },
        ),
    );

    if let Some(resulting_ground_to_field) = pair_association.resulting_ground_to_field {
        let pose = Pose2::new(
            point![
                resulting_ground_to_field.translation().x(),
                resulting_ground_to_field.translation().y()
            ],
            resulting_ground_to_field.orientation().angle(),
        );
        painter.pose(
            pose,
            0.08,
            0.14,
            Color32::TRANSPARENT,
            Stroke::new(0.02, Color32::from_rgb(255, 220, 0)),
        );
    }
}

fn paint_field_mark(painter: &TwixPainter<Field>, field_mark: FieldMark, stroke: Stroke) {
    match field_mark {
        FieldMark::Line { line, .. } => paint_line_segment(painter, line, stroke),
        FieldMark::Circle { center, radius } => {
            painter.circle(center, radius, Color32::TRANSPARENT, stroke);
        }
    }
}

fn paint_line_segment(
    painter: &TwixPainter<Field>,
    line_segment: LineSegment<Field>,
    stroke: Stroke,
) {
    painter.line_segment(line_segment.0, line_segment.1, stroke);
}

fn paint_point(painter: &TwixPainter<Field>, point: Point2<Field>, radius: f32, color: Color32) {
    painter.circle_filled(point, radius, color);
    painter.circle_stroke(point, radius, Stroke::new(0.01, Color32::BLACK));
}

fn confidence_color(score: f32) -> Color32 {
    let normalized_score = (score / 5.0).clamp(0.0, 1.0);
    let red = (255.0 * (1.0 - normalized_score)) as u8;
    let green = (255.0 * normalized_score) as u8;
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
