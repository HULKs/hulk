use std::{str::FromStr, sync::Arc};

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use communication::client::CyclerOutput;
use coordinate_systems::Isometry2;
use types::{
    coordinate_systems::{Field, Ground},
    field_dimensions::FieldDimensions,
};

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct RobotPose {
    ground_to_field: ValueBuffer,
}

impl Layer for RobotPose {
    const NAME: &'static str = "Robot Pose";

    fn new(nao: Arc<Nao>) -> Self {
        let ground_to_field =
            nao.subscribe_output(CyclerOutput::from_str("Control.main.ground_to_field").unwrap());
        Self { ground_to_field }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Field>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        let ground_to_field: Isometry2<Ground, Field> = self.ground_to_field.require_latest()?;

        let pose_color = Color32::from_white_alpha(187);
        let pose_stroke = Stroke {
            width: 0.02,
            color: Color32::BLACK,
        };
        painter.pose(
            ground_to_field.as_pose(),
            0.15,
            0.25,
            pose_color,
            pose_stroke,
        );
        Ok(())
    }
}
