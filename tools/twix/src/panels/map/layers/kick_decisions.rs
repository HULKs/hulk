use std::{str::FromStr, sync::Arc};

use color_eyre::Result;
use communication::client::CyclerOutput;
use eframe::epaint::{Color32, Stroke};

use coordinate_systems::{Field, Ground};
use linear_algebra::{Isometry2, Point2};
use types::{
    field_dimensions::FieldDimensions, kick_decision::KickDecision, kick_target::KickTarget,
};

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct KickDecisions {
    ground_to_field: ValueBuffer,
    kick_decisions: ValueBuffer,
    instant_kick_decisions: ValueBuffer,
    kick_targets: ValueBuffer,
    instant_kick_targets: ValueBuffer,
}

impl Layer for KickDecisions {
    const NAME: &'static str = "Kick Decisions";

    fn new(nao: Arc<Nao>) -> Self {
        let ground_to_field =
            nao.subscribe_output(CyclerOutput::from_str("Control.main.ground_to_field").unwrap());
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
            ground_to_field,
            kick_decisions,
            instant_kick_decisions,
            kick_targets,
            instant_kick_targets,
        }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Field>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        let ground_to_field: Isometry2<Ground, Field> = self.ground_to_field.require_latest()?;
        let kick_decisions: Vec<KickDecision> = self.kick_decisions.require_latest()?;
        let best_kick_decision = kick_decisions.first();
        let instant_kick_decisions: Vec<KickDecision> =
            self.instant_kick_decisions.require_latest()?;
        let kick_targets: Vec<KickTarget> = self.kick_targets.require_latest()?;
        let instant_kick_targets: Vec<Point2<Ground>> =
            self.instant_kick_targets.require_latest()?;

        for kick_decision in &kick_decisions {
            painter.pose(
                ground_to_field * kick_decision.kick_pose,
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
                ground_to_field * kick_decision.kick_pose,
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
                ground_to_field * kick_target.position,
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
                ground_to_field * kick_target,
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
                ground_to_field * kick_decision.kick_pose,
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
