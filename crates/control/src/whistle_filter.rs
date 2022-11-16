use context_attribute::context;
use framework::{MainOutput, Input, Parameter};
use types::{FilteredWhistle, SensorData};

pub struct WhistleFilter {}

#[context]
pub struct NewContext {
    pub buffer_length: Parameter<usize, "control/whistle_filter/buffer_length">,
    pub minimum_detections: Parameter<usize, "control/whistle_filter/minimum_detections">,
}

#[context]
pub struct CycleContext {
    pub sensor_data: Input<SensorData, "sensor_data?">,

    pub buffer_length: Parameter<usize, "control/whistle_filter/buffer_length">,
    pub minimum_detections: Parameter<usize, "control/whistle_filter/minimum_detections">,
    // TODO: wieder einkommentieren
    // pub detected_whistle: PerceptionInput<Whistle, "Audio", "detected_whistle">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub filtered_whistle: MainOutput<FilteredWhistle>,
}

impl WhistleFilter {
    pub fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, _context: CycleContext) -> anyhow::Result<MainOutputs> {
        Ok(MainOutputs::default())
    }
}
