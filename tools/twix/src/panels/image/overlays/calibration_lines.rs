use std::str::FromStr;

use calibration::lines::GoalBoxCalibrationLines;
use color_eyre::Result;
use communication::client::{Cycler, CyclerOutput};
use eframe::epaint::{Color32, Stroke};
use types::Line2;

use crate::{
    panels::image::overlay::Overlay, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct CalibrationLineDetection {
    calibration_line_candidates: ValueBuffer,
    filtered_calibration_lines: ValueBuffer,
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
        }
    }

    fn paint(&self, painter: &TwixPainter) -> Result<()> {
        let lines: Option<Vec<Line2>> = self.calibration_line_candidates.require_latest()?;

        if let Some(lines) = lines {
            for line in lines {
                painter.circle_stroke(line.0, 3.0, Stroke::new(1.0, Color32::RED));
                painter.circle_stroke(line.1, 3.0, Stroke::new(1.0, Color32::RED));
                painter.line_segment(line.0, line.1, Stroke::new(3.0, Color32::BLUE));
            }
            // for (line, reason) in lines_in_image.discarded_lines {
            //     let color = match reason {
            //         types::LineDiscardReason::TooFewPoints => Color32::YELLOW,
            //         types::LineDiscardReason::LineTooShort => Color32::GRAY,
            //         types::LineDiscardReason::LineTooLong => Color32::BROWN,
            //         types::LineDiscardReason::TooFarAway => Color32::BLACK,
            //     };
            //     painter.line_segment(line.0, line.1, Stroke::new(3.0, color));
            // }
        }

        let filtered_calibration_lines: Option<GoalBoxCalibrationLines> =
            self.filtered_calibration_lines.require_latest()?;

        if let Some(filtered_calibration_lines) = filtered_calibration_lines {
            let connecting_line = &filtered_calibration_lines.connecting_line;
            let goal_box_line = &filtered_calibration_lines.goal_box_line;
            let border_line = &filtered_calibration_lines.border_line;

            for line in [connecting_line, goal_box_line, border_line] {
                painter.line_segment(line.0, line.1, Stroke::new(3.0, Color32::GREEN));
            }
        }

        // let lines_in_image: ImageLines = self.calibration_line_candidates.require_latest()?;
        // for point in lines_in_image.points {
        //     painter.circle_stroke(point, 3.0, Stroke::new(1.0, Color32::RED))
        // }
        // for (line, reason) in lines_in_image.discarded_lines {
        //     let color = match reason {
        //         types::LineDiscardReason::TooFewPoints => Color32::YELLOW,
        //         types::LineDiscardReason::LineTooShort => Color32::GRAY,
        //         types::LineDiscardReason::LineTooLong => Color32::BROWN,
        //         types::LineDiscardReason::TooFarAway => Color32::BLACK,
        //     };
        //     painter.line_segment(line.0, line.1, Stroke::new(3.0, color));
        // }
        // for line in lines_in_image.lines {
        //     painter.line_segment(line.0, line.1, Stroke::new(3.0, Color32::BLUE));
        // }
        Ok(())
    }
}
