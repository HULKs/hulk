use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};
use geometry::circle::Circle;
use types::ball::{Ball, CandidateEvaluation};

use crate::{
    panels::image::overlay::{Overlay, VisionCycler},
    value_buffer::ValueBuffer,
};

pub struct BallDetection {
    balls: ValueBuffer,
    filtered_balls: ValueBuffer,
    ball_candidates: ValueBuffer,
}

impl Overlay for BallDetection {
    const NAME: &'static str = "Ball Detection";

    fn new(nao: std::sync::Arc<crate::nao::Nao>, selected_cycler: VisionCycler) -> Self {
        let camera_position = match selected_cycler {
            VisionCycler::VisionTop => "top",
            VisionCycler::VisionBottom => "bottom",
        };
        Self {
            balls: nao.subscribe_output(format!("{}.main_outputs.balls", selected_cycler.to_string())),
            filtered_balls: nao.subscribe_output(format!(
                "Control.additional.filtered_balls_in_image_{}",
                camera_position,
            )),
            ball_candidates: nao.subscribe_output(format!(
                "{}.additional_outputs.ball_candidates",
                selected_cycler.to_string()
            )),
        }
    }

    fn paint(&self, painter: &crate::twix_painter::TwixPainter) -> Result<()> {
        let filtered_balls: Vec<Circle> = self.filtered_balls.require_latest()?;
        for circle in filtered_balls.iter() {
            painter.circle_stroke(circle.center, circle.radius, Stroke::new(3.0, Color32::RED));
        }

        let ball_candidates: Vec<CandidateEvaluation> = self.ball_candidates.require_latest()?;
        for candidate in ball_candidates.iter() {
            let circle = candidate.candidate_circle;
            painter.circle_stroke(
                circle.center,
                circle.radius,
                Stroke::new(2.0, Color32::BLUE),
            );
        }

        let balls: Vec<Ball> = self.balls.require_latest()?;
        for ball in balls.iter() {
            let circle = ball.image_location;
            painter.circle_stroke(
                circle.center,
                circle.radius,
                Stroke::new(2.0, Color32::GREEN),
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

        Ok(())
    }
}
