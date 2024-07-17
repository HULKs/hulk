use std::sync::Arc;

use color_eyre::Result;
use coordinate_systems::Ground;
use eframe::epaint::{Color32, Stroke};
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
        draw_kick_decisions(
            painter,
            &kick_decisions,
            Stroke {
                width: 0.01,
                color: Color32::BLACK,
            },
            Stroke {
                width: 0.01,
                color: Color32::BLACK,
            },
        );
        draw_kick_decisions(
            painter,
            best_kick_decision.cloned().as_slice(),
            Stroke {
                width: 0.02,
                color: Color32::YELLOW,
            },
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
        draw_kick_decisions(
            painter,
            &instant_kick_decisions,
            Stroke {
                width: 0.01,
                color: Color32::RED,
            },
            Stroke {
                width: 0.01,
                color: Color32::RED,
            },
        );
        Ok(())
    }
}

fn draw_kick_decisions<'a>(
    painter: &TwixPainter<Ground>,
    kick_decisions: impl IntoIterator<Item = &'a KickDecision>,
    pose_stroke: Stroke,
    target_stroke: Stroke,
) {
    for kick_decision in kick_decisions {
        painter.pose(
            kick_decision.kick_pose,
            0.05,
            0.1,
            Color32::from_white_alpha(10),
            pose_stroke,
        );
        painter.target(
            kick_decision.target,
            0.1,
            target_stroke,
            Color32::TRANSPARENT,
        )
    }
}
