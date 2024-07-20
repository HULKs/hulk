use color_eyre::Result;
use coordinate_systems::Pixel;
use eframe::epaint::{Color32, Stroke};
use geometry::circle::Circle;
use linear_algebra::vector;
use types::ball_detection::{BallPercept, CandidateEvaluation};

use crate::{
    panels::image::{cycler_selector::VisionCycler, overlay::Overlay},
    twix_painter::TwixPainter,
    value_buffer::BufferHandle,
};

pub struct BallDetection {
    balls: BufferHandle<Option<Vec<BallPercept>>>,
    filtered_balls: BufferHandle<Option<Vec<Circle<Pixel>>>>,
    ball_candidates: BufferHandle<Option<Vec<CandidateEvaluation>>>,
    ball_radius_enlargement_factor: BufferHandle<f32>,
}

impl Overlay for BallDetection {
    const NAME: &'static str = "Ball Detection";

    fn new(nao: std::sync::Arc<crate::nao::Nao>, selected_cycler: VisionCycler) -> Self {
        let camera_position = match selected_cycler {
            VisionCycler::Top => "top",
            VisionCycler::Bottom => "bottom",
        };
        let cycler_path = selected_cycler.as_path();
        let cycler_path_snake_case = selected_cycler.as_snake_case_path();
        Self {
            balls: nao.subscribe_value(format!("{cycler_path}.main_outputs.balls")),
            filtered_balls: nao.subscribe_value(format!(
                "Control.additional_outputs.filtered_balls_in_image_{camera_position}",
            )),
            ball_candidates: nao
                .subscribe_value(format!("{cycler_path}.additional_outputs.ball_candidates")),
            ball_radius_enlargement_factor: nao.subscribe_value(format!(
                "parameters.ball_detection.{cycler_path_snake_case}.ball_radius_enlargement_factor",
            )),
        }
    }

    fn paint(&self, painter: &TwixPainter<Pixel>) -> Result<()> {
        if let Some(filtered_balls) = self.filtered_balls.get_last_value()?.flatten() {
            for circle in &filtered_balls {
                painter.circle_stroke(circle.center, circle.radius, Stroke::new(3.0, Color32::RED));
            }
        }

        if let (Some(ball_candidates), Some(ball_radius_enlargement_factor)) = (
            self.ball_candidates.get_last_value()?.flatten(),
            self.ball_radius_enlargement_factor.get_last_value()?,
        ) {
            for candidate in ball_candidates.iter() {
                let circle = candidate.candidate_circle;
                painter.circle_stroke(
                    circle.center,
                    circle.radius,
                    Stroke::new(1.0, Color32::BLUE),
                );

                let enlarged_candidate = Circle {
                    center: candidate.candidate_circle.center,
                    radius: candidate.candidate_circle.radius * ball_radius_enlargement_factor,
                };
                painter.rect_stroke(
                    enlarged_candidate.center
                        - vector![enlarged_candidate.radius, enlarged_candidate.radius],
                    enlarged_candidate.center
                        + vector![enlarged_candidate.radius, enlarged_candidate.radius],
                    Stroke::new(1.0, Color32::DARK_BLUE),
                );
            }
            for candidate in ball_candidates.iter() {
                if let Some(circle) = candidate.corrected_circle {
                    painter.circle_stroke(
                        circle.center,
                        circle.radius,
                        Stroke::new(1.0, Color32::WHITE),
                    );
                }
            }
        }

        if let Some(balls) = self.balls.get_last_value()?.flatten() {
            for ball in balls.iter() {
                let circle = ball.image_location;
                painter.circle_stroke(
                    circle.center,
                    circle.radius,
                    Stroke::new(2.0, Color32::GREEN),
                );
            }
        }

        Ok(())
    }
}
