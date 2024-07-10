use std::sync::Arc;

use color_eyre::Result;
use coordinate_systems::Ground;
use eframe::epaint::{Color32, Stroke};
use linear_algebra::Point2;
use types::{
    field_dimensions::FieldDimensions, kick_decision::KickDecision,
    kick_target::KickTargetWithKickVariants,
};

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::BufferHandle,
};

pub struct KickDecisions {
    kick_decisions: BufferHandle<Vec<KickDecision>>,
    instant_kick_decisions: BufferHandle<Vec<KickDecision>>,
    kick_opportunities: BufferHandle<Vec<KickTargetWithKickVariants>>,
    instant_kick_targets: BufferHandle<Option<Vec<Point2<Ground>>>>,
}

impl Layer<Ground> for KickDecisions {
    const NAME: &'static str = "Kick Decisions";

    fn new(nao: Arc<Nao>) -> Self {
        let kick_decisions = nao.subscribe_value("Control.main_outputs.kick_decisions");
        let instant_kick_decisions =
            nao.subscribe_value("Control.main_outputs.instant_kick_decisions");
        let kick_opportunities = nao.subscribe_value("Control.main_outputs.kick_opportunities");
        let instant_kick_targets =
            nao.subscribe_value("Control.additional_outputs.instant_kick_targets");
        Self {
            kick_decisions,
            instant_kick_decisions,
            kick_opportunities,
            instant_kick_targets,
        }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Ground>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        self.draw_kick_decisions(painter)?;
        self.draw_instant_kick_decisions(painter)?;
        self.draw_kick_targets(painter)?;
        self.draw_instant_kick_targets(painter)?;

        Ok(())
    }
}

impl KickDecisions {
    fn draw_kick_decisions(&self, painter: &TwixPainter<Ground>) -> Result<()> {
        let Some(kick_decisions) = self.kick_decisions.get_last_value()? else {
            return Ok(());
        };
        let best_kick_decision = kick_decisions.first();
        draw_kick_pose(
            painter,
            &kick_decisions,
            Stroke {
                width: 0.01,
                color: Color32::BLACK,
            },
        );
        draw_kick_pose(
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
        let Some(instant_kick_decisions) = self.instant_kick_decisions.get_last_value()? else {
            return Ok(());
        };
        draw_kick_pose(
            painter,
            &instant_kick_decisions,
            Stroke {
                width: 0.01,
                color: Color32::RED,
            },
        );
        Ok(())
    }

    fn draw_kick_targets(&self, painter: &TwixPainter<Ground>) -> Result<()> {
        let Some(kick_opportunities) = self.kick_opportunities.get_last_value()? else {
            return Ok(());
        };
        draw_kick_target(
            painter,
            kick_opportunities
                .iter()
                .map(|kick_opportunity| kick_opportunity.kick_target.position)
                .collect(),
            Color32::BLACK,
        );
        Ok(())
    }

    fn draw_instant_kick_targets(&self, painter: &TwixPainter<Ground>) -> Result<()> {
        let Some(instant_kick_targets) = self.instant_kick_targets.get_last_value()?.flatten()
        else {
            return Ok(());
        };
        draw_kick_target(painter, instant_kick_targets, Color32::RED);
        Ok(())
    }
}

fn draw_kick_pose(painter: &TwixPainter<Ground>, kick_decisions: &[KickDecision], stroke: Stroke) {
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

fn draw_kick_target(
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
