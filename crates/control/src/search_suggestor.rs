use color_eyre::Result;
use context_attribute::context;
use framework::{AdditionalOutput, MainOutput};
use nalgebra::{DMatrix, Isometry2, Point2};
use serde::{Deserialize, Serialize};
use types::{
    ball_position::{BallPosition, HypotheticalBallPosition},
    field_dimensions::FieldDimensions,
    parameters::SearchSuggestorParameters,
};

#[derive(Deserialize, Serialize)]
pub struct SearchSuggestor {
    heatmap: DMatrix<f32>,
}

#[context]
pub struct CreationContext {
    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    search_suggestor_configuration: Parameter<SearchSuggestorParameters, "search_suggestor">,
}

#[context]
pub struct CycleContext {
    search_suggestor_configuration: Parameter<SearchSuggestorParameters, "search_suggestor">,
    ball_position: Input<Option<BallPosition>, "ball_position?">,
    invalid_ball_positions: Input<Vec<HypotheticalBallPosition>, "invalid_ball_positions">,
    robot_to_field: Input<Option<Isometry2<f32>>, "robot_to_field?">,
    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    heatmap: AdditionalOutput<DMatrix<f32>, "ball_search_heatmap">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub suggested_search_position: MainOutput<Option<Point2<f32>>>,
}

impl SearchSuggestor {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            heatmap: DMatrix::from_element(
                _context.field_dimensions.length as usize
                    * _context.search_suggestor_configuration.cells_per_meter,
                _context.field_dimensions.width as usize
                    * _context.search_suggestor_configuration.cells_per_meter,
                0.0,
            ),
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        self.update_heatmap(
            context.ball_position,
            context.invalid_ball_positions,
            context.robot_to_field.copied(),
            context.search_suggestor_configuration.cells_per_meter,
            context.field_dimensions,
            context.search_suggestor_configuration.heatmap_decay_factor,
        );
        let maximum_heat_heatmap_position = self.heatmap.iamax_full();
        let mut suggested_search_position: Option<Point2<f32>> = None;
        if self.heatmap.get(maximum_heat_heatmap_position).is_some() {
            if self.heatmap[maximum_heat_heatmap_position]
                > context.search_suggestor_configuration.minimum_validity
            {
                let mut search_suggestion = Point2::new(
                    maximum_heat_heatmap_position.0 as f32
                        / context.search_suggestor_configuration.cells_per_meter as f32,
                    maximum_heat_heatmap_position.1 as f32
                        / context.search_suggestor_configuration.cells_per_meter as f32,
                );
                let length_half = context.field_dimensions.length / 2.0;
                let width_half = context.field_dimensions.width / 2.0;

                if search_suggestion.x >= length_half {
                    search_suggestion.x -= length_half;
                } else {
                    search_suggestion.x = length_half - search_suggestion.x
                }
                if search_suggestion.y >= width_half {
                    search_suggestion.y -= width_half;
                } else {
                    search_suggestion.y = width_half - search_suggestion.y
                }
                suggested_search_position = Some(search_suggestion);
            }
        } else {
            println!("Invalid maximum heatmap position")
        }
        context.heatmap.fill_if_subscribed(|| self.heatmap.clone());

        Ok(MainOutputs {
            suggested_search_position: suggested_search_position.into(),
        })
    }

    fn update_heatmap(
        &mut self,
        ball_position: Option<&BallPosition>,
        invalid_ball_positions: &Vec<HypotheticalBallPosition>,
        robot_to_field: Option<Isometry2<f32>>,
        cells_per_meter: usize,
        field_dimensions: &FieldDimensions,
        heatmap_decay_factor: f32,
    ) {
        if let Some(ball_position) = ball_position {
            if let Some(robot_to_field) = robot_to_field {
                let ball_heatmap_position = self.calculate_heatmap_position(
                    robot_to_field * ball_position.position,
                    cells_per_meter,
                    field_dimensions,
                );
                if self.heatmap.get(ball_heatmap_position).is_some() {
                    self.heatmap[ball_heatmap_position] = 1.0;
                } else {
                    println!("Invalid ball heatmap position")
                }
            }
        }
        for ball_hypothesis in invalid_ball_positions {
            let heatmap_position = self.calculate_heatmap_position(
                ball_hypothesis.position,
                cells_per_meter,
                field_dimensions,
            );
            if self.heatmap.get(heatmap_position).is_some() {
                self.heatmap[heatmap_position] =
                    (self.heatmap[heatmap_position] + ball_hypothesis.validity) / 2.0;
            } else {
                println!("Invalid hypothesis heatmap position");
            }
        }
        self.heatmap = self.heatmap.clone() * heatmap_decay_factor;
    }

    fn calculate_heatmap_position(
        &mut self,
        hypothesis_position: Point2<f32>,
        cells_per_meter: usize,
        field_dimensions: &FieldDimensions,
    ) -> (usize, usize) {
        let row_count = field_dimensions.length.round() as usize * cells_per_meter;
        let collum_count = field_dimensions.width.round() as usize * cells_per_meter;
        let mut x_position: usize = 0;
        let mut y_position: usize = 0;
        if hypothesis_position.x > 0.0 {
            x_position =
                (row_count / 2) + (hypothesis_position.x * cells_per_meter as f32).round() as usize;
        } else if hypothesis_position.x < 0.0 {
            x_position = (row_count / 2)
                - (hypothesis_position.x.abs() * cells_per_meter as f32).round() as usize;
        }
        if hypothesis_position.y > 0.0 {
            y_position = (collum_count / 2)
                + (hypothesis_position.y * cells_per_meter as f32).round() as usize;
        } else if hypothesis_position.y < 0.0 {
            y_position = (collum_count / 2)
                - (hypothesis_position.y.abs() * cells_per_meter as f32).round() as usize;
        }
        if row_count < x_position {
            x_position = row_count;
        }
        if collum_count < y_position {
            y_position = collum_count;
        }
        if x_position >= 1 {
            x_position -= 1;
        }
        if y_position >= 1 {
            y_position -= 1;
        }
        (x_position, y_position)
    }
}
