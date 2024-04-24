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
    position: RelativeTime,
}

impl Window {
    pub fn new(
        creation_context: &CreationContext,
        replayer: Replayer<ReplayerHardwareInterface>,
    ) -> Self {
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
            position: RelativeTime::new(0.0),
        }
    }

    fn replay_at_position(&mut self) {
        let frame_range = join_timing(&self.replayer.lock().unwrap());
        let timestamp = self.position.map_to_absolute_time(&frame_range);
        self.time_sender
            .send(timestamp.inner())
            .expect("failed to send replay time");
    }
}

impl App for Window {
    fn update(&mut self, context: &Context, _frame: &mut Frame) {
        let frame_range = join_timing(&self.replayer.lock().unwrap());
        let mut viewport_range = ViewportRange::from_frame_range(&frame_range);

        CentralPanel::default().show(context, |ui| {
            ui.horizontal_top(|ui| {
                ui.add(Labels::new(&self.replayer.lock().unwrap()));
                if ui
                    .add(Timeline::new(
                        &self.replayer.lock().unwrap(),
                        &frame_range,
                        &mut viewport_range,
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
        .unwrap_or(SystemTime::UNIX_EPOCH);
    // .expect("there isn't any index that contains at least one frame");
    let end = recording_indices
        .values()
        .flat_map(|index| {
            index
                .last_timing()
                .map(|timing| timing.timestamp + timing.duration)
        })
        .max()
        .unwrap_or(SystemTime::UNIX_EPOCH);
    // .expect("there isn't any index that contains at least one frame");
    FrameRange::new(AbsoluteTime::new(begin), AbsoluteTime::new(end))
}

fn spawn_replay_thread(
    replayer: Arc<Mutex<Replayer<ReplayerHardwareInterface>>>,
    egui_context: Context,
    mut time: watch::Receiver<SystemTime>,
) {
    spawn(move || {
        let mut progress: f32 = 0.0;
        loop {
            let mut frames_found = false;
            let mut replayer = replayer.lock().unwrap();
            let mut indices = replayer.get_recording_indices_mut();
            for index in indices.values_mut() {
                match index.collect_next_frame_metadata() {
                    Ok(Some(this_progress)) => {
                        progress = progress.min(this_progress);
                        frames_found = true;
                    }
                    Ok(None) => {}
                    Err(error) => eprintln!("{error}"),
                }
            }
            if !frames_found {
                break;
            }
            egui_context.request_repaint();
            // scan one frame
        }
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
