use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use nalgebra::Point2;
use types::{
    ball_position::HypotheticalBallPosition,
    field_dimensions::FieldDimensions,
};

pub struct SearchSuggestor {
    heatmap: Vec<Vec<ProbCell>>,
}

#[context]
pub struct CreationContext {
    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
}

#[context]
pub struct CycleContext {
    removed_ball_positions: Input<Vec<Point2<f32>>, "removed_ball_positions">,
    invalid_ball_positions: Input<Vec<HypotheticalBallPosition>, "invalid_ball_positions">,
}

#[derive(Clone, Copy)]
pub struct ProbCell {
    weight: f64,
    age: f64,
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
            heatmap: vec![
                vec![
                    ProbCell {
                        weight: 0.0,
                        age: 0.0
                    };
                    _context.field_dimensions.length as usize * dpi
                ];
                _context.field_dimensions.width as usize * dpi
            ],
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {


        

        Ok(MainOutputs{
            suggestest_search_position: ,
        }
        )
    }
}
