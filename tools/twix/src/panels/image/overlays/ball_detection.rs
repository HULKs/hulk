use std::str::FromStr;

use color_eyre::Result;
use communication::client::{Cycler, CyclerOutput};
use eframe::epaint::{Color32, Stroke};
use types::{Ball, CandidateEvaluation, Circle};

use crate::{panels::image::overlay::Overlay, value_buffer::ValueBuffer};

pub struct BallDetection {
    balls: ValueBuffer,
    feet: ValueBuffer,
    robot_parts: ValueBuffer,
    penalty_spot: ValueBuffer,
    filtered_balls: ValueBuffer,
    object_candidates: ValueBuffer,
}

impl Overlay for BallDetection {
    const NAME: &'static str = "Ball Detection";

    fn new(nao: std::sync::Arc<crate::nao::Nao>, selected_cycler: Cycler) -> Self {
        let camera_position = match selected_cycler {
            Cycler::VisionTop => "top",
            Cycler::VisionBottom => "bottom",
            cycler => panic!("Invalid vision cycler: {cycler}"),
        };
        Self {
            balls: nao.subscribe_output(
                CyclerOutput::from_str(&format!("{}.main.balls", selected_cycler)).unwrap(),
            ),
            feet: nao.subscribe_output(
                CyclerOutput::from_str(&format!("{}.main.feet", selected_cycler)).unwrap(),
            ),
            robot_parts: nao.subscribe_output(
                CyclerOutput::from_str(&format!("{}.main.robot_part", selected_cycler)).unwrap(),
            ),
            penalty_spot: nao.subscribe_output(
                CyclerOutput::from_str(&format!("{}.main.penalty_spot", selected_cycler)).unwrap(),
            ),
            filtered_balls: nao.subscribe_output(
                CyclerOutput::from_str(&format!(
                    "Control.additional.filtered_balls_in_image_{}",
                    camera_position,
                ))
                .unwrap(),
            ),
            object_candidates: nao.subscribe_output(
                CyclerOutput::from_str(&format!("{}.additional.object_candidates", selected_cycler))
                    .unwrap(),
            ),
        }
    }

    fn paint(&self, painter: &crate::twix_painter::TwixPainter) -> Result<()> {
        let filtered_balls: Vec<Circle> = self.filtered_balls.require_latest()?;
        for circle in filtered_balls.iter() {
            painter.circle_stroke(circle.center, circle.radius, Stroke::new(3.0, Color32::RED));
        }

        let object_candidates: Vec<CandidateEvaluation> = self.object_candidates.require_latest()?;
        for candidate in object_candidates.iter() {
            let circle = candidate.grid_element;
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
        
        let feet: Vec<Ball> = self.feet.require_latest()?;
        for penalty_spot in feet.iter() {
            let circle = penalty_spot.image_location;
            painter.circle_stroke(
                circle.center,
                circle.radius,
                Stroke::new(2.0, Color32::DARK_RED),
            );
        }

        let robot_part: Vec<Ball> = self.robot_parts.require_latest()?;
        for penalty_spot in robot_part.iter() {
            let circle = penalty_spot.image_location;
            painter.circle_stroke(
                circle.center,
                circle.radius,
                Stroke::new(2.0, Color32::GRAY),
            );
        }

        let penalty_spots: Vec<Ball> = self.penalty_spot.require_latest()?;
        for penalty_spot in penalty_spots.iter() {
            let circle = penalty_spot.image_location;
            painter.circle_stroke(
                circle.center,
                circle.radius,
                Stroke::new(2.0, Color32::DARK_GREEN),
            );
        }

        for candidate in object_candidates.iter() {
            if let Some(circle) = candidate.positioned_ball {
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
