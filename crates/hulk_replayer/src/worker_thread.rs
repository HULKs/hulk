use std::{
    thread::spawn,
    time::{Duration, SystemTime},
};

use tokio::{runtime::Builder, select, sync::watch, time::sleep};

use crate::{execution::Replayer, ReplayerHardwareInterface};

pub fn spawn_worker(
    mut replayer: Replayer<ReplayerHardwareInterface>,
    mut time: watch::Receiver<SystemTime>,
) {
    spawn(move || {
        let runtime = Builder::new_current_thread().enable_all().build().unwrap();

        runtime.block_on(async move {
            let parameters_changed = replayer.get_parameters_changed();
            loop {
                select! {
                    _ = parameters_changed.notified() => {}
                    _ = sleep(Duration::from_secs(1)) => {}
                    result = time.changed() => {
                        if result.is_err() {
                            // channel closed, quit thread
                            break;
                        }
                    }
                }

                if let Err(error) = replayer.replay_at(*time.borrow()) {
                    eprintln!("{error:#?}");
                }
            }
        });
    });
}
