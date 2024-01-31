use std::sync::Arc;

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};
use nalgebra::{Isometry2, Point2};
use types::{
    field_dimensions::FieldDimensions,
    multivariate_normal_distribution::MultivariateNormalDistribution,
};

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct BallFilter {
    robot_to_field: ValueBuffer,
    ball_state: ValueBuffer,
}

impl Layer for BallFilter {
    const NAME: &'static str = "Ball Filter";

    fn new(nao: Arc<Nao>) -> Self {
        let robot_to_field = nao.subscribe_output("Control.robot_to_field");
        let ball_hypotheses = nao.subscribe_output("Control.best_ball_state");
        
        Self {
            robot_to_field,
            ball_state: ball_hypotheses,
        }
    }

    fn paint(&self, painter: &TwixPainter, _field_dimensions: &FieldDimensions) -> Result<()> {
        let robot_to_field: Option<Isometry2<f32>> = self.robot_to_field.parse_latest()?;
        let best_ball_state: Option<MultivariateNormalDistribution<4>> =
            self.ball_state.parse_latest()?;

        if let Some(state) = best_ball_state {
            let position = robot_to_field.unwrap_or_default() * Point2::from(state.mean.xy());
            let covariance = state.covariance.fixed_view::<2, 2>(0, 0).into_owned();
            let stroke = Stroke::new(0.01, Color32::BLACK);
            let fill_color = Color32::from_rgba_unmultiplied(255, 255, 0, 100);
            painter.covariance(position, covariance, stroke, fill_color);
        }

        Ok(())
    }
}
