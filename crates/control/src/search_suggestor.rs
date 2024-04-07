use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::{Field, Ground};
use framework::{AdditionalOutput, MainOutput};
use linear_algebra::{point, Isometry2, Point2};
use nalgebra::{clamp, DMatrix, Similarity2, Vector2};
use serde::{Deserialize, Serialize};
use types::{
    ball_position::{BallPosition, HypotheticalBallPosition},
    field_dimensions::FieldDimensions,
    parameters::SearchSuggestorParameters,
};

#[derive(Deserialize, Serialize)]
pub struct SearchSuggestor {
    heatmap: DMatrix<f32>,
    field_to_heatmap: Similarity2<f32>,
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
                * context.search_suggestor_configuration.cells_per_meter as f32)
                .round() as usize,
            (context.field_dimensions.width
                * context.search_suggestor_configuration.cells_per_meter as f32)
                .round() as usize,
        );
        let field_to_heatmap_transformation = Similarity2::new(
            Vector2::new(
                context.field_dimensions.length / 2.0,
                context.field_dimensions.width / 2.0,
            ),
            0.0,
            context.search_suggestor_configuration.cells_per_meter as f32,
        );
        Ok(Self {
            heatmap: DMatrix::from_element(heatmap_length, heatmap_width, 0.0),
            field_to_heatmap: field_to_heatmap_transformation,
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        self.update_heatmap(
            context.ball_position,
            context.invalid_ball_positions,
            context.ground_to_field.copied(),
            context.search_suggestor_configuration.heatmap_decay_factor,
        );
        let maximum_heat_heatmap_position = self.heatmap.iamax_full();
        let mut suggested_search_position: Option<Point2<Field>> = None;

        if self.heatmap[maximum_heat_heatmap_position]
            > context.search_suggestor_configuration.minimum_validity
        {
            let search_position = Vector2::new(
                maximum_heat_heatmap_position.0 as f32,
                maximum_heat_heatmap_position.1 as f32,
            );
            let search_suggestion = self.field_to_heatmap.inverse() * search_position;
            suggested_search_position = Some(point![search_suggestion.x, search_suggestion.y]);
        }
        context.heatmap.fill_if_subscribed(|| self.heatmap.clone());

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
                let ball_field_position = ground_to_field * ball_position.position;
                let ball_heatmap_position = self.field_to_heatmap
                    * Vector2::new(ball_field_position.x(), ball_field_position.y());
                let clamped_ball_heatmap_position = (
                    clamp(
                        ball_heatmap_position.x.round() as usize,
                        0,
                        self.heatmap.shape().0 - 1,
                    ),
                    clamp(
                        ball_heatmap_position.y.round() as usize,
                        0,
                        self.heatmap.shape().1 - 1,
                    ),
                );
                self.heatmap[clamped_ball_heatmap_position] = 1.0;
            }
        }
        for ball_hypothesis in invalid_ball_positions {
            if let Some(ground_to_field) = ground_to_field {
                let ball_hypothesis_field_position = ground_to_field * ball_hypothesis.position;
                let ball_hypothesis_heatmap_position = self.field_to_heatmap
                    * Vector2::new(
                        ball_hypothesis_field_position.x(),
                        ball_hypothesis_field_position.y(),
                    );
                let clamped_ball_heatmap_position = (
                    clamp(
                        ball_hypothesis_heatmap_position.x.round() as usize,
                        0,
                        self.heatmap.shape().0 - 1,
                    ),
                    clamp(
                        ball_hypothesis_heatmap_position.y.round() as usize,
                        0,
                        self.heatmap.shape().1 - 1,
                    ),
                );
                self.heatmap[clamped_ball_heatmap_position] =
                    (self.heatmap[clamped_ball_heatmap_position] + ball_hypothesis.validity) / 2.0;
            }
        }
        self.heatmap.scale_mut(1.0 - heatmap_decay_factor);
    }
}
