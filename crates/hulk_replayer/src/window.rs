use std::{collections::BTreeMap, time::SystemTime};

use blake3::Hash;
use color_eyre::{eyre::ContextCompat, Result};
use eframe::{
    egui::{Align, CentralPanel, Context, Event, Layout, ViewportCommand},
    App, CreationContext, Frame,
};
use tokio::sync::watch;

use framework::Timing;
use tokio_util::sync::CancellationToken;

use crate::{
    bookmarks::Bookmarks,
    controls::Controls,
    coordinate_systems::{AbsoluteTime, FrameRange, RelativeTime, ViewportRange},
    labels::Labels,
    timeline::Timeline,
    worker_thread::PlayerState,
};

pub struct Window {
    replay_identifier: Hash,
    bookmarks: Bookmarks,
    controls: Controls,
    time_sender: watch::Sender<PlayerState>,
    frame_range: FrameRange,
    viewport_range: ViewportRange,
    indices: BTreeMap<String, Vec<Timing>>,
    cancellation_token: CancellationToken,
}

impl Window {
    pub fn new(
        context: &CreationContext,
        replay_identifier: Hash,
        indices: BTreeMap<String, Vec<Timing>>,
        time_sender: watch::Sender<PlayerState>,
        cancellation_token: CancellationToken,
    ) -> Result<Self> {
        let frame_range = join_timing(&indices);
        let viewport_range = ViewportRange::from_frame_range(&frame_range);

        let storage = context
            .storage
            .wrap_err("failed to access persistent storage")?;

        let bookmarks = storage
            .get_string(&format!("replay_{}", replay_identifier.to_hex()))
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_else(|| Bookmarks {
                latest: frame_range.start(),
                bookmarks: BTreeMap::new(),
            });

        time_sender.send_modify(|state| state.time = bookmarks.latest.inner());

        Ok(Self {
            replay_identifier,
            bookmarks,
            controls: Controls::default(),
            time_sender,
            frame_range,
            viewport_range,
            indices,
            cancellation_token,
        })
    }

    fn replay_at_position(&mut self, position: RelativeTime) {
        let timestamp = position.map_to_absolute_time(&self.frame_range);
        self.time_sender
            .send_modify(|state| state.time = timestamp.inner())
    }
}

impl App for Window {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let bookmarks =
            serde_json::to_string(&self.bookmarks).expect("failed to serialize bookmarks");
        storage.set_string(
            &format!("replay_{}", self.replay_identifier.to_hex()),
            bookmarks,
        );
    }

    fn update(&mut self, context: &Context, _frame: &mut Frame) {
        if self.cancellation_token.is_cancelled() {
            log::info!("shutdown ui");
            context.send_viewport_cmd(ViewportCommand::Close);
        }
        let absolute_position = AbsoluteTime::new(self.time_sender.borrow().time);

        context.input_mut(|input| {
            if input.consume_shortcut(&self.controls.play_pause) {
                self.time_sender
                    .send_modify(|state| state.playing = !state.playing);
            }
            if input.consume_shortcut(&self.controls.create_bookmark) {
                self.bookmarks.add(absolute_position)
            }
            if input.consume_shortcut(&self.controls.delete_bookmark) {
                self.bookmarks.remove_if_exists(&absolute_position);
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
            ui.horizontal_top(|ui| {
                ui.label(format!(
                    "Speed: {}",
                    self.time_sender.borrow().playback_rate
                ));
                ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                    ui.menu_button("?", |ui| {
                        ui.add(&self.controls);
                    });
                });
            });
            ui.horizontal_top(|ui| {
                ui.add(Labels::new(&self.indices));
                let mut relative_position =
                    absolute_position.map_to_relative_time(&self.frame_range);
                if ui
                    .add(Timeline::new(
                        &self.controls,
                        &self.indices,
                        &self.frame_range,
                        &mut self.viewport_range,
                        &mut relative_position,
                        &mut self.bookmarks,
                    ))
                    .changed()
                {
                    self.bookmarks.latest =
                        relative_position.map_to_absolute_time(&self.frame_range);
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
