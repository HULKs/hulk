use std::ops::{Index, IndexMut};

use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::{Field, Ground};
use framework::{AdditionalOutput, MainOutput};
use linear_algebra::{point, Isometry2, Point2};
use nalgebra::{clamp, DMatrix};
use serde::{Deserialize, Serialize};
use types::{
    ball_position::{BallPosition, HypotheticalBallPosition},
    field_dimensions::FieldDimensions,
    parameters::SearchSuggestorParameters,
};

#[derive(Deserialize, Serialize)]
pub struct SearchSuggestor {
    heatmap: Heatmap,
}

#[context]
pub struct CreationContext {
    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    search_suggestor_configuration: Parameter<SearchSuggestorParameters, "search_suggestor">,
}

#[context]
pub struct CycleContext {
    search_suggestor_configuration: Parameter<SearchSuggestorParameters, "search_suggestor">,
    ball_position: Input<Option<BallPosition<Ground>>, "ball_position?">,
    invalid_ball_positions: Input<Vec<HypotheticalBallPosition<Ground>>, "invalid_ball_positions">,
    ground_to_field: Input<Option<Isometry2<Ground, Field>>, "ground_to_field?">,
    heatmap: AdditionalOutput<DMatrix<f32>, "ball_search_heatmap">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub suggested_search_position: MainOutput<Option<Point2<Field>>>,
}

impl SearchSuggestor {
    pub fn new(context: CreationContext) -> Result<Self> {
        let (heatmap_length, heatmap_width) = (
            (context.field_dimensions.length
                * context.search_suggestor_configuration.cells_per_meter)
                .round() as usize,
            (context.field_dimensions.width
                * context.search_suggestor_configuration.cells_per_meter)
                .round() as usize,
        );
        let heatmap = Heatmap {
            map: DMatrix::from_element(heatmap_length, heatmap_width, 0.0),
            field_dimensions: context.field_dimensions.clone(),
            cells_per_meter: context.search_suggestor_configuration.cells_per_meter,
        };
        Ok(Self { heatmap })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        self.update_heatmap(
            context.ball_position,
            context.invalid_ball_positions,
            context.ground_to_field.copied(),
            context.search_suggestor_configuration.heatmap_decay_factor,
        );
        let suggested_search_position = self
            .heatmap
            .get_maximum_position(context.search_suggestor_configuration.minimum_validity);

        context
            .heatmap
            .fill_if_subscribed(|| self.heatmap.map.clone());

        Ok(MainOutputs {
            suggested_search_position: suggested_search_position.into(),
        })
    }

    fn update_heatmap(
        &mut self,
        ball_position: Option<&BallPosition<Ground>>,
        invalid_ball_positions: &Vec<HypotheticalBallPosition<Ground>>,
        ground_to_field: Option<Isometry2<Ground, Field>>,
        heatmap_decay_factor: f32,
    ) {
        if let Some(ball_position) = ball_position {
            if let Some(ground_to_field) = ground_to_field {
                self.heatmap[ground_to_field * ball_position.position] = 1.0;
            }
        }
        for ball_hypothesis in invalid_ball_positions {
            if let Some(ground_to_field) = ground_to_field {
                let ball_hypothesis_position = ground_to_field * ball_hypothesis.position;
                self.heatmap[ball_hypothesis_position] =
                    (self.heatmap[ball_hypothesis_position] + ball_hypothesis.validity) / 2.0;
            }
        }
        self.heatmap.map.scale_mut(1.0 - heatmap_decay_factor);
    }
}

#[derive(Deserialize, Serialize)]
struct Heatmap {
    map: DMatrix<f32>,
    field_dimensions: FieldDimensions,
    cells_per_meter: f32,
}

impl Heatmap {
    fn field_to_heatmap(&self, field_point: Point2<Field>) -> (usize, usize) {
        let heatmap_point = (
            ((field_point.x() + self.field_dimensions.length / 2.0) * self.cells_per_meter)
                as usize,
            ((field_point.y() + self.field_dimensions.width / 2.0) * self.cells_per_meter) as usize,
        );
        (
            clamp(heatmap_point.0, 0, self.map.shape().0 - 1),
            clamp(heatmap_point.1, 0, self.map.shape().1 - 1),
        )
    }

    fn get_maximum_position(&self, minimum_validity: f32) -> Option<Point2<Field>> {
        let maximum_heat_heatmap_position = self.map.iamax_full();

        if self.map[maximum_heat_heatmap_position] > minimum_validity {
            let search_suggestion = point![
                ((maximum_heat_heatmap_position.0 as f32 + 1.0 / 2.0) / self.cells_per_meter
                    - self.field_dimensions.length / 2.0),
                ((maximum_heat_heatmap_position.1 as f32 + 1.0 / 2.0) / self.cells_per_meter
                    - self.field_dimensions.width / 2.0)
            ];
            return Some(search_suggestion);
        }
        None
    }
}

impl Index<Point2<Field>> for Heatmap {
    type Output = f32;
    fn index(&self, field_point: Point2<Field>) -> &Self::Output {
        let heatmap_point = self.field_to_heatmap(field_point);
        &self.map[heatmap_point]
    }
}

impl IndexMut<Point2<Field>> for Heatmap {
    fn index_mut(&mut self, field_point: Point2<Field>) -> &mut Self::Output {
        let heatmap_point = self.field_to_heatmap(field_point);
        &mut self.map[heatmap_point]
    }
}
