use std::{collections::VecDeque, time::SystemTime};

use anyhow::Result;
use module_derive::{module, require_some};
use types::{FilteredWhistle, SensorData, Whistle};

pub struct WhistleFilter {
    pub detection_buffer: VecDeque<bool>,
    pub was_detected_last_cycle: bool,
    pub last_detection: Option<SystemTime>,
}

#[module(control)]
#[input(path = sensor_data, data_type = SensorData)]
#[perception_input(path = detected_whistle, data_type = Whistle, cycler = audio)]
#[parameter(path = control.whistle_filter.buffer_length, data_type = usize)]
#[parameter(path = control.whistle_filter.minimum_detections, data_type = usize)]
#[main_output(data_type = FilteredWhistle)]
impl WhistleFilter {}

impl WhistleFilter {
    fn new(_context: NewContext) -> anyhow::Result<Self> {
        Ok(Self {
            detection_buffer: Default::default(),
            was_detected_last_cycle: false,
            last_detection: None,
        })
    }

    fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let cycle_start_time = require_some!(context.sensor_data).cycle_info.start_time;

        for is_detected in context
            .detected_whistle
            .persistent
            .values()
            .flatten()
            .filter_map(|&detected_whistle| {
                detected_whistle.as_ref().map(|whistle| whistle.is_detected)
            })
            .flatten()
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
            filtered_whistle: Some(FilteredWhistle {
                is_detected,
                last_detection: self.last_detection,
                started_this_cycle,
            }),
        })
    }
}
