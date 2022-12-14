use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use types::FieldColor;

pub struct FieldColorDetection {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub field_color: MainOutput<FieldColor>,
}

impl FieldColorDetection {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
