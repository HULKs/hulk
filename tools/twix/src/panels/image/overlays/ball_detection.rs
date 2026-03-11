use color_eyre::Result;
use coordinate_systems::Pixel;
use eframe::epaint::{Color32, Stroke};
use geometry::circle::Circle;

use crate::{
    panels::image::overlay::Overlay, twix_painter::TwixPainter, value_buffer::BufferHandle,
};

pub struct BallDetection {
    filtered_balls: BufferHandle<Option<Vec<Circle<Pixel>>>>,
}

impl Overlay for BallDetection {
    const NAME: &'static str = "Ball Detection";

    fn new(robot: std::sync::Arc<crate::robot::Robot>) -> Self {
        Self {
            filtered_balls: robot
                .subscribe_value("WorldState.additional_outputs.filtered_balls_in_image"),
        }
    }

    fn paint(&self, painter: &TwixPainter<Pixel>) -> Result<()> {
        if let Some(filtered_balls) = self.filtered_balls.get_last_value()?.flatten() {
            for circle in &filtered_balls {
                painter.circle_stroke(circle.center, circle.radius, Stroke::new(3.0, Color32::RED));
            }
        }

        Ok(())
    }
}
