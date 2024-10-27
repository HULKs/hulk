use std::{net::IpAddr, time::Duration};

use aliveness::query_aliveness;
use eframe::egui::Context;
use tokio::{
    runtime::{Builder, Runtime},
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
};

pub struct ReachableNaos {
    ips: Vec<IpAddr>,
    tx: UnboundedSender<Vec<IpAddr>>,
    rx: UnboundedReceiver<Vec<IpAddr>>,
    context: Context,
    runtime: Runtime,
}

impl ReachableNaos {
    pub fn new(context: Context) -> Self {
        let ips = Vec::new();
        let (tx, rx) = unbounded_channel();
        let runtime = Builder::new_multi_thread().enable_all().build().unwrap();

        Self {
            ips,
            tx,
            rx,
            context,
            runtime,
        }
    }

    pub fn query_reachability(&self) {
        let tx = self.tx.clone();
        let context = self.context.clone();
        self.runtime.spawn(async move {
            if let Ok(ips) = query_aliveness(Duration::from_millis(200), None).await {
                let ips = ips.into_iter().map(|(ip, _)| ip).collect();
                let _ = tx.send(ips);
                context.request_repaint();
            }
        });
    }

    pub fn update(&mut self) {
        while let Ok(ips) = self.rx.try_recv() {
            self.ips = ips;
        }
    }

    pub fn is_reachable(&self, ip: impl Into<IpAddr>) -> bool {
        self.ips.contains(&ip.into())
    }
}
