use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use types::{FieldBorder, FilteredSegments, ImageSegments};

pub struct SegmentFilter {}

#[context]
pub struct NewContext {}

#[context]
pub struct CycleContext {
    pub field_border: RequiredInput<Option<FieldBorder>, "field_border?">,
    pub image_segments: RequiredInput<Option<ImageSegments>, "image_segments?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub filtered_segments: MainOutput<Option<FilteredSegments>>,
}

impl SegmentFilter {
    pub fn new(_context: NewContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
