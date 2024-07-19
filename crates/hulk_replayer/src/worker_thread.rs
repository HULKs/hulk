use std::{
    thread::spawn,
    time::{Duration, SystemTime},
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
            playing: Default::default(),
            playback_rate: 1.0,
        }
    }
}

pub fn spawn_worker(
    mut replayer: Replayer<ReplayerHardwareInterface>,
    sender: watch::Sender<PlayerState>,
    update_callback: impl Fn() + Send + Sync + 'static,
) {
    spawn(move || {
        let runtime = Builder::new_current_thread().enable_all().build().unwrap();

        runtime.block_on(async move {
            let mut receiver = sender.subscribe();
            let mut parameters_receiver = replayer.get_parameters_receiver();
            loop {
                select! {
                    _ = parameters_receiver.wait_for_change() => {}
                    _ = sleep(Duration::from_secs(1)) => {}
                    result = receiver.changed() => {
                        if result.is_err() {
                            // channel closed, quit thread
                            break;
                        }
                    }
                }

                let state = *receiver.borrow();
                if state.playing {
                    let sender = sender.clone();
                    tokio::spawn(async move {
                        sleep(Duration::from_millis(12)).await;
                        sender.send_modify(|state| {
                            state.time += Duration::from_secs_f32(0.012 * state.playback_rate)
                        });
                    });
                }

                if let Err(error) = replayer.replay_at(state.time) {
                    eprintln!("{error:#?}");
                }

                update_callback()
            }
        });
    });
}
