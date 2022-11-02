use std::{sync::Arc, time::Duration};

use context_attribute::context;
use framework::{AdditionalOutput, MainOutput, OptionalInput, Parameter};
use types::{
    configuration::{EdgeDetectionSource, MedianMode},
    CameraMatrix, FieldColor, Image422, ImageSegments, ProjectedLimbs,
};

pub struct ImageReceiver {}

#[context]
pub struct NewContext {}

#[context]
pub struct CycleContext {}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub image: MainOutput<Arc<Image422>>,
}

impl ImageReceiver {
    pub fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
