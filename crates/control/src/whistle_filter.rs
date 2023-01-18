use std::{collections::VecDeque, time::SystemTime};

use color_eyre::Result;
use context_attribute::context;
use framework::{MainOutput, PerceptionInput};
use types::{CycleTime, FilteredWhistle, SensorData, Whistle};

pub struct WhistleFilter {
    pub detection_buffer: VecDeque<bool>,
    pub was_detected_last_cycle: bool,
    pub last_detection: Option<SystemTime>,
}

#[context]
pub struct CreationContext {
    pub buffer_length: Parameter<usize, "control.whistle_filter.buffer_length">,
    pub minimum_detections: Parameter<usize, "control.whistle_filter.minimum_detections">,
}

#[context]
pub struct CycleContext {
    pub sensor_data: Input<SensorData, "sensor_data">,
    pub cycle_time: Input<CycleTime, "cycle_time">,

    pub buffer_length: Parameter<usize, "control.whistle_filter.buffer_length">,
    pub minimum_detections: Parameter<usize, "control.whistle_filter.minimum_detections">,
    pub detected_whistle: PerceptionInput<Whistle, "Audio", "detected_whistle">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub filtered_whistle: MainOutput<FilteredWhistle>,
}

impl WhistleFilter {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            detection_buffer: Default::default(),
            was_detected_last_cycle: false,
            last_detection: None,
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let cycle_start_time = context.cycle_time.start_time;

        for is_detected in context
            .detected_whistle
            .persistent
            .values()
            .flatten()
            .flat_map(|whistle| whistle.is_detected)
        {
            self.detection_buffer.push_front(is_detected);
        }
        self.detection_buffer.truncate(*context.buffer_length);
        let number_of_detections = self
            .detection_buffer
            .iter()
            .filter(|&&was_detected| was_detected)
            .count();
        let is_detected = number_of_detections > *context.minimum_detections;
        let started_this_cycle = is_detected && !self.was_detected_last_cycle;
        if started_this_cycle {
            self.last_detection = Some(cycle_start_time);
        }
        self.was_detected_last_cycle = is_detected;

        Ok(MainOutputs {
            filtered_whistle: FilteredWhistle {
                is_detected,
                last_detection: self.last_detection,
                started_this_cycle,
            }
            .into(),
        })
    }
}
