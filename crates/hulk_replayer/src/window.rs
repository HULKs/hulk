use std::{collections::BTreeMap, time::SystemTime};

use eframe::{
    egui::{CentralPanel, Context},
    App, Frame,
};
use tokio::sync::watch;

use framework::Timing;

use crate::{
    coordinate_systems::{AbsoluteTime, FrameRange, RelativeTime, ViewportRange},
    labels::Labels,
    timeline::Timeline,
};

pub struct Window {
    time_sender: watch::Sender<SystemTime>,
    frame_range: FrameRange,
    viewport_range: ViewportRange,
    position: RelativeTime,
    indices: BTreeMap<String, Vec<Timing>>,
}

impl Window {
    pub fn new(
        indices: BTreeMap<String, Vec<Timing>>,
        time_sender: watch::Sender<SystemTime>,
    ) -> Self {
        let frame_range = join_timing(&indices);
        let viewport_range = ViewportRange::from_frame_range(&frame_range);

        Self {
            time_sender,
            frame_range,
            viewport_range,
            position: RelativeTime::new(0.0),
            indices,
        }
    }

    fn replay_at_position(&mut self) {
        let timestamp = self.position.map_to_absolute_time(&self.frame_range);
        self.time_sender
            .send(timestamp.inner())
            .expect("failed to send replay time");
    }
}

impl App for Window {
    fn update(&mut self, context: &Context, _frame: &mut Frame) {
        CentralPanel::default().show(context, |ui| {
            ui.horizontal_top(|ui| {
                ui.add(Labels::new(&self.indices));
                if ui
                    .add(Timeline::new(
                        &self.indices,
                        &self.frame_range,
                        &mut self.viewport_range,
                        &mut self.position,
                    ))
                    .changed()
                {
                    self.replay_at_position();
                }
            });
        });
    }
}

fn join_timing(indices: &BTreeMap<String, Vec<Timing>>) -> FrameRange {
    let begin = indices
        .values()
        .flat_map(|index| index.first().map(|timing| timing.timestamp))
        .min()
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let end = indices
        .values()
        .flat_map(|index| {
            index
                .last()
                .map(|timing| timing.timestamp + timing.duration)
        })
        .max()
        .unwrap_or(SystemTime::UNIX_EPOCH);
    FrameRange::new(AbsoluteTime::new(begin), AbsoluteTime::new(end))
}
