use color_eyre::Result;
use context_attribute::context;
use framework::{AdditionalOutput, MainOutput};
use types::{CameraMatrix, FilteredSegments, ImageLines, LineData};

pub struct LineDetection {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    pub lines_in_image: AdditionalOutput<ImageLines, "lines_in_image">,

    pub camera_matrix: RequiredInput<Option<CameraMatrix>, "camera_matrix?">,
    pub filtered_segments: RequiredInput<Option<FilteredSegments>, "filtered_segments?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub line_data: MainOutput<Option<LineData>>,
}

impl LineDetection {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
