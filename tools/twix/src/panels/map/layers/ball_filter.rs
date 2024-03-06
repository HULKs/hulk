use std::sync::Arc;

use color_eyre::Result;
use eframe::epaint::{Color32, Stroke};

use communication::client::{Cycler, CyclerOutput, Output};
use linear_algebra::{Isometry2, Point};
use types::{
    coordinate_systems::{Field, Ground},
    field_dimensions::FieldDimensions,
    multivariate_normal_distribution::MultivariateNormalDistribution,
};

use crate::{
    nao::Nao, panels::map::layer::Layer, twix_painter::TwixPainter, value_buffer::ValueBuffer,
};

pub struct BallFilter {
    ground_to_field: ValueBuffer,
    ball_state: ValueBuffer,
}

impl Layer for BallFilter {
    const NAME: &'static str = "Ball Filter";

    fn new(nao: Arc<Nao>) -> Self {
        let ground_to_field = nao.subscribe_output(CyclerOutput {
            cycler: Cycler::Control,
            output: Output::Main {
                path: "ground_to_field".to_string(),
            },
        });
        let ball_state = nao.subscribe_output(CyclerOutput {
            cycler: Cycler::Control,
            output: Output::Additional {
                path: "best_ball_state".to_string(),
            },
        });
        Self {
            ground_to_field,
            ball_state,
        }
    }

    fn paint(
        &self,
        painter: &TwixPainter<Field>,
        _field_dimensions: &FieldDimensions,
    ) -> Result<()> {
        let ground_to_field: Option<Isometry2<Ground, Field>> =
            self.ground_to_field.parse_latest()?;
        let ball_state: Option<MultivariateNormalDistribution<4>> =
            self.ball_state.parse_latest()?;

        if let Some(state) = ball_state {
            let position = ground_to_field.unwrap_or_default() * Point::from(state.mean.xy());
            let covariance = state.covariance.fixed_view::<2, 2>(0, 0).into_owned();
            let stroke = Stroke::new(0.01, Color32::BLACK);
            let fill_color = Color32::from_rgba_unmultiplied(255, 255, 0, 100);
            painter.covariance(position, covariance, stroke, fill_color);
        }

        Ok(())
    }
}
