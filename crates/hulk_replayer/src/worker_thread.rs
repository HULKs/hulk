use std::{
    thread::spawn,
    time::{Duration, Instant, SystemTime},
};

use tokio::{runtime::Builder, select, sync::watch, time::sleep};

use crate::{execution::Replayer, ReplayerHardwareInterface};

#[derive(Clone, Copy)]
pub struct PlayerState {
    pub time: SystemTime,
    pub playing: bool,
    pub playback_rate: f32,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            time: SystemTime::UNIX_EPOCH,
            playing: false,
            playback_rate: 1.0,
        }
    }
}

pub fn spawn_workers(
    replayer: Replayer<ReplayerHardwareInterface>,
    sender: watch::Sender<PlayerState>,
    update_callback: impl Fn() + Send + Sync + 'static,
) {
    spawn(move || {
        let runtime = Builder::new_current_thread().enable_all().build().unwrap();

        runtime.spawn(playback_worker(sender.clone()));
        runtime.block_on(replay_worker(replayer, sender.subscribe(), update_callback));
    });
}

async fn replay_worker(
    mut replayer: Replayer<ReplayerHardwareInterface>,
    mut receiver: watch::Receiver<PlayerState>,
    update_callback: impl Fn() + Send + Sync + 'static,
) {
    let mut parameters_receiver = replayer.get_parameters_receiver();
    loop {
        select! {
            _ = parameters_receiver.wait_for_change() => {}
            _ = sleep(Duration::from_secs(1)) => {}
            result = receiver.changed() => {
                if result.is_err() {
                    // channel closed, quit
                    break;
                }
            }
        }

        let timestamp = receiver.borrow().time;
        if let Err(error) = replayer.replay_at(timestamp) {
            eprintln!("{error:#?}");
        }

        update_callback()
    }
}

async fn playback_worker(sender: watch::Sender<PlayerState>) {
    let mut receiver = sender.subscribe();
    let mut last_autoplay_time = None;
    loop {
        select! {
            _ = receiver.changed() => {
                let state = *receiver.borrow();
                if !state.playing {
                    last_autoplay_time = None
                }
            }
            _ = sleep(Duration::from_millis(12)), if receiver.borrow().playing => {
                let elapsed = last_autoplay_time
                    .as_ref()
                    .map(Instant::elapsed)
                    .unwrap_or(Duration::from_millis(12));
                last_autoplay_time = Some(Instant::now());
                sender.send_modify(|state| {
                    if state.playback_rate.is_sign_positive(){
                        state.time += elapsed.mul_f32(state.playback_rate);
                    } else {
                        state.time -= elapsed.mul_f32(-state.playback_rate);
                    }
                });
            }
        }
    }
}
