use std::sync::Arc;

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};
use nalgebra::{Isometry2, Point2};
use types::{field_dimensions::FieldDimensions, pose_detection::HumanPose};

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct RefereePosition {
    expected_referee_position: ValueBuffer,
    field_width: ValueBuffer,
    threshhold: ValueBuffer,
    // injected_robot_to_field_of_home_after_coin_toss_before_second_half: ValueBuffer,
}

impl Layer for RefereePosition {
    const NAME: &'static str = "Referee Position";

    fn new(nao: Arc<Nao>) -> Self {
        let expected_referee_position =
            nao.subscribe_parameter("detection.detection_top.expected_referee_position");
        let field_width = nao.subscribe_parameter("field_dimensions.width");
        let threshhold = nao
            .subscribe_parameter("detection.detection_top.distance_to_referee_position_threshhold");
        // let injected_robot_to_field_of_home_after_coin_toss_before_second_half = nao
        //     .subscribe_parameter(
        //         "injected_robot_to_field_of_home_after_coin_toss_before_second_half",
        //     );
        Self {
            expected_referee_position,
            field_width,
            threshhold,
            // injected_robot_to_field_of_home_after_coin_toss_before_second_half,
        }
    }

    fn paint(&self, painter: &TwixPainter, _field_dimensions: &FieldDimensions) -> Result<()> {
        let position_stroke = Stroke {
            width: 0.05,
            color: Color32::BLACK,
        };

        let expected_referee_position: Point2<f32> =
            self.expected_referee_position.require_latest()?;
        let field_width: f32 = self.field_width.require_latest()?;
        painter.circle(
            expected_referee_position * field_width / 2.0,
            0.15,
            Color32::BLUE,
            position_stroke,
        );

        let threshhold: f32 = self.threshhold.require_latest()?;
        painter.circle(
            expected_referee_position * field_width / 2.0,
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
