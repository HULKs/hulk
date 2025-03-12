use std::sync::Arc;

use crate::{
    nao::Nao,
    panels::image::{cycler_selector::VisionCycler, overlay::Overlay},
    twix_painter::TwixPainter,
};
use calibration::goal_and_penalty_box::LineType;
use color_eyre::Result;
use coordinate_systems::Pixel;
use eframe::egui::{Align2, Color32, Stroke};
use geometry::line_segment::LineSegment;
use linear_algebra::point;

pub struct LineTest {
    lines: Vec<(LineType, LineSegment<Pixel>)>,
}

impl Overlay for LineTest {
    const NAME: &'static str = "Line Test";

    fn new(_nao: Arc<Nao>, _selected_cycler: VisionCycler) -> Self {
        let lines = vec![
            (
                LineType::Goal,
                LineSegment::new(point![86.0, 196.0], point![640.0, 212.0]),
            ),
            (
                LineType::LeftPenaltyArea,
                LineSegment::new(point![7.2, 250.0], point![176.0, 198.0]),
            ),
            (
                LineType::FrontPenaltyArea,
                LineSegment::new(point![7.2, 250.0], point![640.0, 285.0]),
            ),
            (
                LineType::LeftGoalArea,
                LineSegment::new(point![253.0, 216.0], point![280.0, 203.0]),
            ),
        ];
        Self { lines }
    }

    fn paint(&self, painter: &TwixPainter<Pixel>) -> Result<()> {
        for (line_type, line) in &self.lines {
            painter.line_segment(line.0, line.1, Stroke::new(2.0, Color32::DARK_BLUE));
            painter.floating_text(
                line.center(),
                Align2::CENTER_CENTER,
                format!("{line_type:?}"),
                Default::default(),
                Color32::BLACK,
            );
        }
        Ok(())
    }
}
