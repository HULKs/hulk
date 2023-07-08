use std::str::FromStr;

use crate::{
    panels::image::overlay::Overlay, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};
use calibration::lines::GoalBoxCalibrationLines;
use color_eyre::Result;
use communication::client::{Cycler, CyclerOutput};
use coordinate_systems::Pixel;
use eframe::epaint::{Color32, Stroke};
use geometry::line::Line2;
use linear_algebra::Point2;

pub struct CalibrationLineDetection {
    calibration_line_candidates: ValueBuffer,
    filtered_calibration_lines: ValueBuffer,
    circle_used_points: ValueBuffer,
}

impl Overlay for CalibrationLineDetection {
    const NAME: &'static str = "Calibration Line Detection";

    fn new(nao: std::sync::Arc<crate::nao::Nao>, selected_cycler: Cycler) -> Self {
        Self {
            calibration_line_candidates: nao.subscribe_output(
                CyclerOutput::from_str(&format!(
                    "{selected_cycler}.additional.calibration_line_detection.unfiltered_lines"
                ))
                .unwrap(),
            ),
            filtered_calibration_lines: nao.subscribe_output(
                CyclerOutput::from_str(&format!(
                    "{selected_cycler}.main.calibration_line_detection"
                ))
                .unwrap(),
            ),
            circle_used_points: nao.subscribe_output(
                CyclerOutput::from_str(&format!(
                    "{selected_cycler}.additional.calibration_line_detection.circle_used_points"
                ))
                .unwrap(),
            ),
        }
    }

    fn paint(&self, painter: &TwixPainter<Pixel>) -> Result<()> {
        // let lines: Option<Vec<Line2<Pixel>>> = ?; =  self.calibration_line_candidates.require_latest();
        // if let Ok(lines) = self.calibration_line_candidates.require_latest() {
        //     for line in lines {
        //         painter.circle_stroke(line.0, 3.0, Stroke::new(1.0, Color32::RED));
        //         painter.circle_stroke(line.1, 3.0, Stroke::new(1.0, Color32::RED));
        //         painter.line_segment(line.0, line.1, Stroke::new(3.0, Color32::BLUE));
        //     }
        // }

        // let _filtered_calibration_lines: Option<GoalBoxCalibrationLines<Pixel>> =
        //     self.filtered_calibration_lines.require_latest()?;

        // if let Ok(filtered_calibration_lines) = self.filtered_calibration_lines.require_latest() {
        //     let connecting_line = &filtered_calibration_lines.connecting_line;
        //     let goal_box_line = &filtered_calibration_lines.goal_box_line;
        //     let border_line = &filtered_calibration_lines.border_line;

        //     for line in [connecting_line, goal_box_line, border_line] {
        //         painter.line_segment(line.0, line.1, Stroke::new(3.0, Color32::GREEN));
        //     }
        // }

        let used_points: Vec<Point2<Pixel>> = self.circle_used_points.require_latest()?;

        for circle_point in used_points {
            painter.circle_stroke(circle_point, 2.0, Stroke::new(1.0, Color32::YELLOW));
        }

        Ok(())
    }
}
