use std::{str::FromStr, sync::Arc};

use color_eyre::Result;
use communication::client::CyclerOutput;
use eframe::epaint::{Color32, Stroke};
use nalgebra::{Isometry2, Point2};
use types::FieldDimensions;

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct KickDecisions {
    robot_to_field: ValueBuffer,
    kick_decisions: ValueBuffer,
    kick_targets: ValueBuffer,
}

impl Layer for KickDecisions {
    const NAME: &'static str = "Kick Decisions";

    fn new(nao: Arc<Nao>) -> Self {
        let robot_to_field =
            nao.subscribe_output(CyclerOutput::from_str("control.main.robot_to_field").unwrap());
        let kick_decisions = nao
            .subscribe_output(CyclerOutput::from_str("control.additional.kick_decisions").unwrap());
        let kick_targets = nao
            .subscribe_output(CyclerOutput::from_str("control.additional.kick_targets").unwrap());
        Self {
            robot_to_field,
            kick_decisions,
            kick_targets,
        }
    }

    fn paint(&self, painter: &TwixPainter, _field_dimensions: &FieldDimensions) -> Result<()> {
        let robot_to_field: Isometry2<f32> = self.robot_to_field.require_latest()?;
        let kick_decisions: Vec<types::KickDecision> = self.kick_decisions.require_latest()?;
        let kick_targets: Vec<Point2<f32>> = self.kick_targets.require_latest()?;

        for kick_decision in kick_decisions {
            painter.pose(
                robot_to_field * kick_decision.relative_kick_pose,
                0.05,
                0.1,
                Color32::from_white_alpha(10),
                Stroke {
                    width: 0.01,
                    color: Color32::BLACK,
                },
            );
        }

        for kick_target in kick_targets {
            painter.circle_stroke(
                robot_to_field * kick_target,
                0.1,
                Stroke {
                    width: 0.01,
                    color: Color32::BLUE,
                },
            )
        }
        Ok(())
    }
}
