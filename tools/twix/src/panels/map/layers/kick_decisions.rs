use std::{str::FromStr, sync::Arc};

use color_eyre::Result;
use communication::client::CyclerOutput;
use eframe::epaint::{Color32, Stroke};

use coordinate_systems::Ground;
use linear_algebra::Point2;
use types::{
    field_dimensions::FieldDimensions, kick_decision::KickDecision, kick_target::KickTarget,
};

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct KickDecisions {
    kick_decisions: ValueBuffer,
    instant_kick_decisions: ValueBuffer,
    kick_targets: ValueBuffer,
    instant_kick_targets: ValueBuffer,
}

impl Layer<Ground> for KickDecisions {
    const NAME: &'static str = "Kick Decisions";

    fn new(nao: Arc<Nao>) -> Self {
        let kick_decisions =
            nao.subscribe_output(CyclerOutput::from_str("Control.main.kick_decisions").unwrap());
        let instant_kick_decisions = nao.subscribe_output(
            CyclerOutput::from_str("Control.main.instant_kick_decisions").unwrap(),
        );
        let kick_targets = nao
            .subscribe_output(CyclerOutput::from_str("Control.additional.kick_targets").unwrap());
        let instant_kick_targets = nao.subscribe_output(
            CyclerOutput::from_str("Control.additional.instant_kick_targets").unwrap(),
        );
        Self {
            kick_decisions,
            instant_kick_decisions,
            kick_targets,
            instant_kick_targets,
        }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Ground>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        let kick_decisions: Vec<KickDecision> = self.kick_decisions.require_latest()?;
        let best_kick_decision = kick_decisions.first();
        let instant_kick_decisions: Vec<KickDecision> =
            self.instant_kick_decisions.require_latest()?;
        let kick_targets: Vec<KickTarget> = self.kick_targets.require_latest()?;
        let instant_kick_targets: Vec<Point2<Ground>> =
            self.instant_kick_targets.require_latest()?;

        for kick_decision in &kick_decisions {
            painter.pose(
                kick_decision.kick_pose,
                0.05,
                0.1,
                Color32::from_white_alpha(10),
                Stroke {
                    width: 0.01,
                    color: Color32::BLACK,
                },
            );
        }
        for kick_decision in &instant_kick_decisions {
            painter.pose(
                kick_decision.kick_pose,
                0.05,
                0.1,
                Color32::from_white_alpha(10),
                Stroke {
                    width: 0.01,
                    color: Color32::RED,
                },
            );
        }

        for kick_target in kick_targets {
            painter.target(
                kick_target.position,
                0.1,
                Stroke {
                    width: 0.01,
                    color: Color32::BLACK,
                },
                Color32::TRANSPARENT,
            )
        }
        for kick_target in instant_kick_targets {
            painter.target(
                kick_target,
                0.1,
                Stroke {
                    width: 0.01,
                    color: Color32::RED,
                },
                Color32::TRANSPARENT,
            )
        }
        if let Some(kick_decision) = best_kick_decision {
            painter.pose(
                kick_decision.kick_pose,
                0.05,
                0.1,
                Color32::from_white_alpha(10),
                Stroke {
                    width: 0.02,
                    color: Color32::YELLOW,
                },
            );
        }
        Ok(())
    }
}
