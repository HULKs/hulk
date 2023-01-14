use std::{str::FromStr, sync::Arc};

use color_eyre::Result;
use communication::client::CyclerOutput;
use eframe::epaint::{Color32, Stroke};
use nalgebra::Isometry2;
use types::{FieldDimensions, MotionCommand, PathSegment};

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct Path {
    robot_to_field: ValueBuffer,
    motion_command: ValueBuffer,
}

impl Layer for Path {
    const NAME: &'static str = "Path";

    fn new(nao: Arc<Nao>) -> Self {
        let robot_to_field =
            nao.subscribe_output(CyclerOutput::from_str("Control.main.robot_to_field").unwrap());
        let motion_command =
            nao.subscribe_output(CyclerOutput::from_str("Control.main.motion_command").unwrap());
        Self {
            robot_to_field,
            motion_command,
        }
    }

    fn paint(&self, painter: &TwixPainter, _field_dimensions: &FieldDimensions) -> Result<()> {
        let robot_to_field: Isometry2<f32> = self.robot_to_field.require_latest()?;
        let motion_command: MotionCommand = self.motion_command.require_latest()?;

        if let MotionCommand::Walk { path, .. } = motion_command {
            for segment in path {
                match segment {
                    PathSegment::LineSegment(line_segment) => painter.line_segment(
                        robot_to_field * line_segment.0,
                        robot_to_field * line_segment.1,
                        Stroke {
                            width: 0.025,
                            color: Color32::BLUE,
                        },
                    ),
                    PathSegment::Arc(arc, orientation) => painter.arc(
                        arc,
                        orientation,
                        Stroke {
                            width: 0.025,
                            color: Color32::LIGHT_BLUE,
                        },
                        robot_to_field,
                    ),
                }
            }
        }
        Ok(())
    }
}
