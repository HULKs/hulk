use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use nalgebra::{DMatrix, Point2};
use types::{
    ball_position::HypotheticalBallPosition, field_dimensions::FieldDimensions,
    parameters::SearchSuggestorParameters,
};

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
    //removed_ball_positions: Input<Vec<Point2<f32>>, "removed_ball_positions">,
    search_suggestor_configuration: Parameter<SearchSuggestorParameters, "search_suggestor">,
    invalid_ball_positions: Input<Vec<HypotheticalBallPosition>, "invalid_ball_positions">,
    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub suggestest_search_position: MainOutput<Point2<f32>>,
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

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        self.update_heatmap(
            context.invalid_ball_positions,
            context.search_suggestor_configuration.cells_per_meter,
            context.field_dimensions,
            context.search_suggestor_configuration.heatmap_decay_factor,
        );
        let maximum_heat_heatmap_position = self.heatmap.iamax_full();
        let suggestest_search_position = Point2::new(
            maximum_heat_heatmap_position.0 as f32
                / context.search_suggestor_configuration.cells_per_meter as f32,
            maximum_heat_heatmap_position.1 as f32
                / context.search_suggestor_configuration.cells_per_meter as f32,
        );

        Ok(MainOutputs {
            suggestest_search_position: suggestest_search_position.into(),
        })
    }

    fn update_heatmap(
        &mut self,
        invalid_ball_positions: &Vec<HypotheticalBallPosition>,
        cells_per_meter: usize,
        field_dimensions: &FieldDimensions,
        heatmap_decay_factor: f32,
    ) {
        for ball_hypothesis in invalid_ball_positions {
            let heatmap_position = self.calculate_heatmap_position(
                ball_hypothesis.position,
                cells_per_meter,
                field_dimensions,
            );
            self.heatmap[heatmap_position] += ball_hypothesis.validity;
        }
        self.heatmap = self.heatmap.clone() * heatmap_decay_factor;
    }

    fn calculate_heatmap_position(
        &mut self,
        hypothesis_position: Point2<f32>,
        cells_per_meter: usize,
        field_dimensions: &FieldDimensions,
    ) -> (usize, usize) {
        let row_count = field_dimensions.length as usize * cells_per_meter;
        let collum_count = field_dimensions.width as usize * cells_per_meter;
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
        return (x_position, y_position);
    }
}
