use context_attribute::context;
use framework::{MainOutput, OptionalInput};

pub struct SegmentFilter {}

#[context]
pub struct NewContext {}

#[context]
pub struct CycleContext {
    pub field_border: OptionalInput<FieldBorder, "field_border">,
    pub image_segments: OptionalInput<ImageSegments, "image_segments">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub filtered_segments: MainOutput<FilteredSegments>,
}

impl SegmentFilter {
    pub fn new(context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
