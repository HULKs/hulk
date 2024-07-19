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
            let mut last_autoplay_time = Instant::now();
            loop {
                select! {
                    _ = parameters_receiver.wait_for_change() => {}
                    _ = sleep(Duration::from_secs(1)) => {}
                    _ = sleep(Duration::from_millis(12)), if receiver.borrow().playing => {
                        let elapsed = last_autoplay_time.elapsed();
                        last_autoplay_time = Instant::now();
                        sender.send_modify(|state| {
                            state.time += elapsed.mul_f32(state.playback_rate);
                        });
                        // receiver is updated anyway, prevent duplicate replaying
                        continue;
                    }
                    result = receiver.changed() => {
                        if result.is_err() {
                            // channel closed, quit thread
                            break;
                        }
                    }
                }

                let state = *receiver.borrow();

                if let Err(error) = replayer.replay_at(state.time) {
                    eprintln!("{error:#?}");
                }

                update_callback()
            }
        });
    });
}
