use std::{collections::BTreeMap, time::SystemTime};

use eframe::{
    egui::{CentralPanel, Context, Event, Key, Modifiers, ViewportCommand},
    App, Frame,
};
use tokio::sync::watch;

use framework::Timing;
use tokio_util::sync::CancellationToken;

use crate::{
    coordinate_systems::{AbsoluteTime, FrameRange, RelativeTime, ViewportRange},
    labels::Labels,
    timeline::Timeline,
    worker_thread::PlayerState,
};

pub struct Window {
    time_sender: watch::Sender<PlayerState>,
    frame_range: FrameRange,
    viewport_range: ViewportRange,
    indices: BTreeMap<String, Vec<Timing>>,
    cancellation_token: CancellationToken,
}

impl Window {
    pub fn new(
        indices: BTreeMap<String, Vec<Timing>>,
        time_sender: watch::Sender<PlayerState>,
        cancellation_token: CancellationToken,
    ) -> Self {
        let frame_range = join_timing(&indices);
        let viewport_range = ViewportRange::from_frame_range(&frame_range);
        time_sender.send_modify(|state| state.time = frame_range.start().inner());

        Self {
            time_sender,
            frame_range,
            viewport_range,
            indices,
            cancellation_token,
        }
    }

    fn replay_at_position(&mut self, position: RelativeTime) {
        let timestamp = position.map_to_absolute_time(&self.frame_range);
        self.time_sender
            .send_modify(|state| state.time = timestamp.inner())
    }
}

impl App for Window {
    fn update(&mut self, context: &Context, _frame: &mut Frame) {
        if self.cancellation_token.is_cancelled() {
            log::info!("shutdown ui");
            context.send_viewport_cmd(ViewportCommand::Close);
        }
        context.input_mut(|input| {
            if input.consume_key(Modifiers::NONE, Key::Space) {
                self.time_sender
                    .send_modify(|state| state.playing = !state.playing);
            }
            if input
                .events
                .iter()
                // egui is scheiße
                .any(|e| *e == Event::Text("<".to_string()))
            {
                self.time_sender
                    .send_modify(|state| state.playback_rate -= 0.25);
            }
            if input
                .events
                .iter()
                // egui is scheiße
                .any(|e| *e == Event::Text(">".to_string()))
            {
                self.time_sender
                    .send_modify(|state| state.playback_rate += 0.25);
            }
        });
        CentralPanel::default().show(context, |ui| {
            ui.label(format!(
                "Speed: {}",
                self.time_sender.borrow().playback_rate
            ));
            ui.horizontal_top(|ui| {
                ui.add(Labels::new(&self.indices));
                let absolute_position = AbsoluteTime::new(self.time_sender.borrow().time);
                let mut relative_position =
                    absolute_position.map_to_relative_time(&self.frame_range);
                if ui
                    .add(Timeline::new(
                        &self.indices,
                        &self.frame_range,
                        &mut self.viewport_range,
                        &mut relative_position,
                    ))
                    .changed()
                {
                    self.replay_at_position(relative_position);
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
