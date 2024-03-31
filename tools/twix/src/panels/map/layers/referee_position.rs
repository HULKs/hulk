use std::sync::Arc;

use color_eyre::Result;
use coordinate_systems::{Field, Ground};
use eframe::epaint::{Color32, Stroke};
use linear_algebra::{Isometry2, Point2};

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct RefereePosition {
    expected_referee_position: ValueBuffer,
    threshhold: ValueBuffer,
    ground_to_field: ValueBuffer,
    // injected_robot_to_field_of_home_after_coin_toss_before_second_half: ValueBuffer,
}

impl Layer for RefereePosition {
    const NAME: &'static str = "Referee Position";

    fn new(nao: Arc<Nao>) -> Self {
        let expected_referee_position =
            nao.subscribe_parameter("control.expected_referee_position");
        let threshhold = nao
            .subscribe_parameter("detection.detection_top.distance_to_referee_position_threshhold");
        let ground_to_field = nao.subscribe_parameter("control.ground_to_field");
        // let injected_robot_to_field_of_home_after_coin_toss_before_second_half = nao
        //     .subscribe_parameter(
        //         "injected_robot_to_field_of_home_after_coin_toss_before_second_half",
        //     );
        Self {
            expected_referee_position,
            threshhold,
            ground_to_field,
            // injected_robot_to_field_of_home_after_coin_toss_before_second_half,
        }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Field>,
        _field_dimension: &types::field_dimensions::FieldDimensions,
    ) -> Result<()> {
        let position_stroke = Stroke {
            width: 0.05,
            color: Color32::BLACK,
        };

        let ground_to_field: Isometry2<Ground, Field> = self.ground_to_field.require_latest()?;

        let expected_referee_position: Point2<Ground> =
            self.expected_referee_position.require_latest()?;
        painter.circle(
            ground_to_field * expected_referee_position,
            0.15,
            Color32::BLUE,
            position_stroke,
        );

        let threshhold: f32 = self.threshhold.require_latest()?;
        painter.circle(
            ground_to_field * expected_referee_position,
            threshhold,
            Color32::TRANSPARENT,
            position_stroke,
        );

        // Not the correct point for the expected referee position, are there any other
        // exisiting resources that give that position adapted to the current playing side?

        // let injected_robot_to_field_of_home_after_coin_toss_before_second_half: Isometry2<f32> =
        //     self.injected_robot_to_field_of_home_after_coin_toss_before_second_half
        //         .require_latest()?;
        // let other_referee_position =
        //     injected_robot_to_field_of_home_after_coin_toss_before_second_half * Point2::origin();
        // painter.circle(other_referee_position, 0.15, Color32::RED, position_stroke);

        Ok(())
    }
}
