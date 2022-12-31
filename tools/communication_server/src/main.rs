use std::{
    sync::{Arc, Barrier},
    thread::sleep,
    time::Duration,
};

use communication::server::Server;
use ctrlc::set_handler;
use tokio_util::sync::CancellationToken;

fn main() {
    println!("Foo");
    let keep_running = CancellationToken::new();
    let barrier = Arc::new(Barrier::new(2));
    {
        let keep_running = keep_running.clone();
        let barrier = barrier.clone();
        set_handler(move || {
            keep_running.cancel();
            barrier.wait();
        })
        .expect("failed to set handler");
    }
    let _server = Server::start(keep_running).expect("failed to start server");
    barrier.wait();
    sleep(Duration::from_secs(1));
}
