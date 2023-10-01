use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use nalgebra::{Point2, DMatrix,};
use types::{
    ball_position::HypotheticalBallPosition,
    field_dimensions::FieldDimensions, ball,
};

pub struct SearchSuggestor {
    heatmap: DMatrix<f32>,
}

#[context]
pub struct CreationContext {
    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
}

#[context]
pub struct CycleContext {
    removed_ball_positions: Input<Vec<Point2<f32>>, "removed_ball_positions">,
    invalid_ball_positions: Input<Vec<HypotheticalBallPosition>, "invalid_ball_positions">,
    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub suggestest_search_position: MainOutput<Point2<f64>>,
}

impl SearchSuggestor {
    pub fn new(_context: CreationContext) -> Result<Self> {
        let dpi:usize = 5;
        Ok(Self {
            heatmap: DMatrix::from_element(_context.field_dimensions.length as usize * dpi,  _context.field_dimensions.width as usize * dpi, 0.0),
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {

        self.update_heatmap(context.invalid_ball_positions, context.field_dimensions);
        

        Ok(MainOutputs{
            suggestest_search_position: ,
        }
        )
    }

    fn update_heatmap(&mut self, invalid_ball_positions: &Vec<HypotheticalBallPosition>, field_dimensions: &FieldDimensions,){
        let dpi:usize = 5;
        for ball_hypothesis in invalid_ball_positions{
            self.heatmap[self.calculate_heatmap_position(ball_hypothesis.position, dpi, field_dimensions)] = ball_hypothesis.validity;
        }
    }
    
    fn calculate_heatmap_position(&mut self, hypothesis_position: Point2<f32>, dpi: usize, field_dimensions: &FieldDimensions)-> (usize, usize){
        let row_count = field_dimensions.length as usize * dpi;
        let collum_count = field_dimensions.width as usize * dpi;
        let mut x_position: usize = 0;
        let mut y_position: usize = 0;
        if hypothesis_position.x > 0.0 {
            x_position = (row_count/2)  + (hypothesis_position.x * dpi as f32).round() as usize;
        } else if hypothesis_position.x < 0.0 {
            x_position = (hypothesis_position.x.abs() * dpi as f32).round() as usize;     
        }
        if hypothesis_position.y > 0.0 {
            y_position = (row_count/2)  + (hypothesis_position.y * dpi as f32).round() as usize;
        } else if hypothesis_position.y < 0.0 {
            y_position = (hypothesis_position.y.abs() * dpi as f32).round() as usize;     
        }
        return(x_position, y_position);
    }
}
