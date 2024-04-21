use std::collections::BTreeMap;

use eframe::{
    egui::{CentralPanel, Context},
    App, Frame,
};

use crate::{
    coordinate_systems::{AbsoluteTime, FrameRange, RelativeTime, ViewportRange},
    execution::Replayer,
    labels::Labels,
    timeline::Timeline,
    ReplayerHardwareInterface,
};

pub struct Window {
    replayer: Replayer<ReplayerHardwareInterface>,
    frame_range: FrameRange,
    viewport_range: ViewportRange,
    position: RelativeTime,
}

impl Window {
    pub fn new(replayer: Replayer<ReplayerHardwareInterface>) -> Self {
        let frame_range = join_timing(&replayer);
        let viewport_range = ViewportRange::from_frame_range(&frame_range);
        Self {
            replayer,
            frame_range,
            viewport_range,
            position: RelativeTime::new(0.0),
        }
    }

    fn replay_at_position(&mut self) {
        let timestamp = self.position.map_to_absolute_time(&self.frame_range);
        let recording_indices = self.replayer.get_recording_indices_mut();
        let frames = recording_indices
            .into_iter()
            .map(|(name, index)| {
                (
                    name,
                    index
                        .find_latest_frame_up_to(timestamp.inner())
                        .expect("failed to find latest frame"),
                )
            })
            .collect::<BTreeMap<_, _>>();
        for (name, frame) in frames {
            if let Some(frame) = frame {
                self.replayer
                    .replay(&name, frame.timing.timestamp, &frame.data)
                    .expect("failed to replay frame");
            }
        }
    }
}

impl App for Window {
    fn update(&mut self, context: &Context, _frame: &mut Frame) {
        CentralPanel::default().show(context, |ui| {
            ui.horizontal_top(|ui| {
                ui.add(Labels::new(&self.replayer));
                if ui
                    .add(Timeline::new(
                        &self.replayer,
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

fn join_timing(replayer: &Replayer<ReplayerHardwareInterface>) -> FrameRange {
    let recording_indices = replayer.get_recording_indices();
    let begin = recording_indices
        .values()
        .flat_map(|index| index.first_timing().map(|timing| timing.timestamp))
        .min()
        .expect("there isn't any index that contains at least one frame");
    let end = recording_indices
        .values()
        .flat_map(|index| {
            index
                .last_timing()
                .map(|timing| timing.timestamp + timing.duration)
        })
        .max()
        .expect("there isn't any index that contains at least one frame");
    FrameRange::new(AbsoluteTime::new(begin), AbsoluteTime::new(end))
}
