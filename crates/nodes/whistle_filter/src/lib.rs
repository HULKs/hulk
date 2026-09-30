use std::{boxed::Box, future::Future, pin::Pin};
use std::{collections::VecDeque, sync::Arc, time::SystemTime};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use ros_z::prelude::*;
use types::{filtered_whistle::FilteredWhistle, whistle::Whistle};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub buffer_length: usize,
    pub minimum_detections: usize,
}

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("whistle_filter").build().await?;

    let parameters = node.bind_parameter_as::<Parameters>("whistle_filter")?;
    let detected_whistle_sub = node
        .subscriber::<Whistle>("detected_whistle")
        .build()
        .await?;
    let filtered_whistle_pub = node
        .publisher::<FilteredWhistle>("filtered_whistle")
        .build()
        .await?;

    let mut detection_buffer = VecDeque::new();
    let mut was_detected_last_cycle = false;
    let mut last_detection = None;

    loop {
        let detected_whistle = detected_whistle_sub.recv().await?;
        let parameters_snapshot = parameters.snapshot();
        let parameters = parameters_snapshot.typed();

        for is_detected in detected_whistle.is_detected {
            detection_buffer.push_front(is_detected);
        }
        detection_buffer.truncate(parameters.buffer_length);

        let number_of_detections = detection_buffer
            .iter()
            .filter(|&&is_detected| is_detected)
            .count();
        let is_detected = number_of_detections >= parameters.minimum_detections;
        if is_detected && !was_detected_last_cycle {
            last_detection = Some(SystemTime::now());
        }
        was_detected_last_cycle = is_detected;

        filtered_whistle_pub
            .publish(&FilteredWhistle {
                is_detected,
                last_detection,
            })
            .await?;
    }
}
