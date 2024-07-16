use std::sync::Arc;

use color_eyre::Result;
use coordinate_systems::Ground;
use eframe::epaint::{Color32, Stroke};
use linear_algebra::Point2;
use types::{field_dimensions::FieldDimensions, kick_decision::KickDecision};

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::BufferHandle,
};

pub struct KickDecisions {
    kick_decisions: BufferHandle<Option<Vec<KickDecision>>>,
    instant_kick_decisions: BufferHandle<Option<Vec<KickDecision>>>,
}

impl Layer<Ground> for KickDecisions {
    const NAME: &'static str = "Kick Decisions";

    fn new(nao: Arc<Nao>) -> Self {
        let kick_decisions = nao.subscribe_value("Control.main_outputs.kick_decisions");
        let instant_kick_decisions =
            nao.subscribe_value("Control.main_outputs.instant_kick_decisions");
        Self {
            kick_decisions,
            instant_kick_decisions,
        }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Ground>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        self.draw_kick_decisions(painter)?;
        self.draw_instant_kick_decisions(painter)?;

        Ok(())
    }
}

impl KickDecisions {
    fn draw_kick_decisions(&self, painter: &TwixPainter<Ground>) -> Result<()> {
        let Some(kick_decisions) = self.kick_decisions.get_last_value()?.flatten() else {
            return Ok(());
        };
        let best_kick_decision = kick_decisions.first();
        draw_kick_targets(
            painter,
            kick_decisions
                .iter()
                .map(|decision| decision.target)
                .collect(),
            Color32::BLACK,
        );
        draw_kick_poses(
            painter,
            &kick_decisions,
            Stroke {
                width: 0.01,
                color: Color32::BLACK,
            },
        );
        draw_kick_poses(
            painter,
            best_kick_decision.cloned().as_slice(),
            Stroke {
                width: 0.02,
                color: Color32::YELLOW,
            },
        );
        Ok(())
    }

    fn draw_instant_kick_decisions(&self, painter: &TwixPainter<Ground>) -> Result<()> {
        let Some(instant_kick_decisions) = self.instant_kick_decisions.get_last_value()?.flatten()
        else {
            return Ok(());
        };
        draw_kick_targets(
            painter,
            instant_kick_decisions
                .iter()
                .map(|decision| decision.target)
                .collect(),
            Color32::RED,
        );
        draw_kick_poses(
            painter,
            &instant_kick_decisions,
            Stroke {
                width: 0.01,
                color: Color32::RED,
            },
        );
        Ok(())
    }
}

fn draw_kick_poses(painter: &TwixPainter<Ground>, kick_decisions: &[KickDecision], stroke: Stroke) {
    for kick_decision in kick_decisions {
        painter.pose(
            kick_decision.kick_pose,
            0.05,
            0.1,
            Color32::from_white_alpha(10),
            stroke,
        );
    }
}

fn draw_kick_targets(
    painter: &TwixPainter<Ground>,
    kick_targets: Vec<Point2<Ground>>,
    stroke_color: Color32,
) {
    for kick_target in kick_targets {
        painter.target(
            kick_target,
            0.1,
            Stroke {
                width: 0.01,
                color: stroke_color,
            },
            Color32::TRANSPARENT,
        )
    }
}
