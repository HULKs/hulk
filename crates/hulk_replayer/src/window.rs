use std::{
    sync::{Arc, Mutex},
    thread::spawn,
    time::SystemTime,
};

use eframe::{
    egui::{CentralPanel, Context},
    App, CreationContext, Frame,
};
use tokio::{runtime::Builder, select, sync::watch};

use crate::{
    coordinate_systems::{AbsoluteTime, FrameRange, RelativeTime, ViewportRange},
    execution::Replayer,
    labels::Labels,
    timeline::Timeline,
    ReplayerHardwareInterface,
};

pub struct Window {
    replayer: Arc<Mutex<Replayer<ReplayerHardwareInterface>>>,
    time_sender: watch::Sender<SystemTime>,
    frame_range: FrameRange,
    viewport_range: ViewportRange,
    position: RelativeTime,
}

impl Window {
    pub fn new(
        creation_context: &CreationContext,
        replayer: Replayer<ReplayerHardwareInterface>,
    ) -> Self {
        let frame_range = join_timing(&replayer);
        let viewport_range = ViewportRange::from_frame_range(&frame_range);

        let replayer = Arc::new(Mutex::new(replayer));
        let (time_sender, time_receiver) = watch::channel(SystemTime::UNIX_EPOCH);
        spawn_replay_thread(
            replayer.clone(),
            creation_context.egui_ctx.clone(),
            time_receiver,
        );

        Self {
            replayer,
            time_sender,
            frame_range,
            viewport_range,
            position: RelativeTime::new(0.0),
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
                ui.add(Labels::new(&self.replayer.lock().unwrap()));
                if ui
                    .add(Timeline::new(
                        &self.replayer.lock().unwrap(),
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

fn spawn_replay_thread(
    replayer: Arc<Mutex<Replayer<ReplayerHardwareInterface>>>,
    egui_context: Context,
    mut time: watch::Receiver<SystemTime>,
) {
    spawn(move || {
        let runtime = Builder::new_current_thread().build().unwrap();

        runtime.block_on(async move {
            let parameters_changed = replayer.lock().unwrap().get_parameters_changed();
            loop {
                select! {
                    _ = parameters_changed.notified() => {
                    }
                    result = time.changed() => {
                        if result.is_err() {
                            // channel closed, quit thread
                            break;
                        }
                    }
                }

                let _ = replayer.lock().unwrap().replay_at(*time.borrow());
                egui_context.request_repaint();
            }
        });
    });
}
