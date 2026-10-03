use std::time::{Duration, Instant};

use color_eyre::eyre::{Result, eyre};
use eframe::egui::Context;
use ros_z::node::Node;
use ros_z_debug::replay::{ReplayCommand, ReplayStatus, control_key};
use tokio::{sync::oneshot, task::JoinHandle};

use crate::backend::RobotBackend;

const POLL_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Default)]
pub struct ReplaySession {
    namespace: String,
    pub status: Option<ReplayStatus>,
    pub error: Option<String>,
    pending: Option<ReplayCommand>,
    request: Option<(JoinHandle<()>, oneshot::Receiver<Result<ReplayStatus>>)>,
    command_in_flight: bool,
    next_request: Option<Instant>,
    target_position: Option<u64>,
}

impl ReplaySession {
    pub fn position(&self) -> Option<u64> {
        self.target_position
            .or_else(|| self.status.as_ref().map(|status| status.position))
    }

    pub fn is_seeking(&self) -> bool {
        self.target_position.is_some()
    }

    pub fn seek(&mut self, position: u64) {
        if self.error.is_some() {
            return;
        }
        if let Some(status) = &self.status {
            let position = position.clamp(status.start, status.end);
            if self.target_position == Some(position)
                || (self.target_position.is_none()
                    && position == status.position
                    && !status.playing)
            {
                return;
            }
            self.target_position = Some(position);
            self.pending = Some(ReplayCommand::Seek {
                instance: status.instance.clone(),
                position,
            });
        }
    }

    pub fn set_playing(&mut self, playing: bool) {
        if self.error.is_some() || self.is_seeking() {
            return;
        }
        if let Some(status) = &self.status {
            let instance = status.instance.clone();
            self.pending = Some(if playing {
                ReplayCommand::Play { instance }
            } else {
                ReplayCommand::Pause { instance }
            });
        }
    }

    pub fn retry(&mut self) {
        self.next_request = None;
    }

    // Shared by all timeline panes. Run even when they are hidden or closed:
    // other clients can seek, and every observation must follow the new generation.
    pub fn update(&mut self, context: &Context, backend: &RobotBackend) {
        let namespace = backend.namespace();
        if self.namespace != namespace {
            if let Some((task, _)) = self.request.take() {
                task.abort();
            }
            if self.status.is_some() {
                backend.set_replay_sources(None);
            }
            *self = Self::default();
            self.namespace = namespace;
        }
        let result = self
            .request
            .as_mut()
            .and_then(|(_, receiver)| match receiver.try_recv() {
                Ok(result) => Some(result),
                Err(oneshot::error::TryRecvError::Empty) => None,
                Err(oneshot::error::TryRecvError::Closed) => {
                    Some(Err(eyre!("replay request stopped")))
                }
            });
        if let Some(result) = result {
            self.request = None;
            self.command_in_flight = false;
            match result {
                Ok(status) => {
                    let changed = self.status.as_ref().is_none_or(|old| {
                        old.instance != status.instance || old.generation != status.generation
                    });
                    if changed {
                        backend.set_replay_sources(Some(status.sources.clone()));
                    }
                    if self
                        .status
                        .as_ref()
                        .is_some_and(|old| old.instance != status.instance)
                    {
                        self.pending = None;
                        self.target_position = None;
                    }
                    self.status = Some(status);
                    self.error = None;
                    if self.pending.is_none() {
                        self.target_position = None;
                    }
                }
                Err(error) => {
                    self.error = Some(format!("{error:#}"));
                    self.pending = None;
                    self.target_position = None;
                }
            }
            self.next_request = Some(
                Instant::now()
                    + if self.error.is_some() {
                        Duration::from_secs(1)
                    } else {
                        POLL_INTERVAL
                    },
            );
        }

        context.request_repaint_after(if self.status.is_some() {
            POLL_INTERVAL
        } else {
            Duration::from_secs(1)
        });
    }

    /// Called after panels handle input: commands leave in the same frame and
    /// bypass status polling delays. Only the newest unsent seek is retained.
    pub fn dispatch(&mut self, context: &Context, backend: &RobotBackend) {
        if self.pending.is_some()
            && !self.command_in_flight
            && let Some((task, _)) = self.request.take()
        {
            task.abort(); // Only a read-only status query may be superseded.
        }
        if self.request.is_none() && self.request_due(Instant::now()) {
            let node = backend.node();
            let key = control_key(&self.namespace);
            let command = self.pending.take();
            self.command_in_flight = command.is_some();
            let context = context.clone();
            let (sender, receiver) = oneshot::channel();
            let task = backend.runtime_handle().spawn(async move {
                let result = request(&node, key, command).await;
                let _ = sender.send(result);
                context.request_repaint();
            });
            self.request = Some((task, receiver));
        }
    }

    fn request_due(&self, now: Instant) -> bool {
        self.pending.is_some() || self.next_request.is_none_or(|deadline| now >= deadline)
    }
}

impl Drop for ReplaySession {
    fn drop(&mut self) {
        if let Some((task, _)) = &self.request {
            task.abort();
        }
    }
}

async fn request(node: &Node, key: String, command: Option<ReplayCommand>) -> Result<ReplayStatus> {
    let mut query = node.session().get(key).timeout(Duration::from_secs(3));
    if let Some(command) = command {
        query = query.payload(serde_json::to_vec(&command)?);
    }
    let replies = query.await.map_err(|error| eyre!(error))?;
    let reply = replies
        .recv_async()
        .await
        .map_err(|_| eyre!("replay server unavailable"))?;
    let sample = reply
        .into_result()
        .map_err(|error| eyre!("{}", String::from_utf8_lossy(&error.payload().to_bytes())))?;
    let status: ReplayStatus = serde_json::from_slice(&sample.payload().to_bytes())?;
    color_eyre::eyre::ensure!(
        status.start <= status.position
            && status.position <= status.end
            && status.end <= i64::MAX as u64,
        "invalid playback time bounds"
    );
    Ok(status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrubbing_coalesces_and_clamps_without_losing_the_final_seek() {
        let mut replay = ReplaySession::default();
        replay.status = Some(ReplayStatus {
            instance: "recording".into(),
            recording: "test.mcap".into(),
            generation: 1,
            start: 100,
            end: 200,
            position: 150,
            playing: false,
            sources: Default::default(),
        });
        for position in [120, 180, 0, 160, u64::MAX] {
            replay.seek(position);
        }
        assert_eq!(replay.position(), Some(200));
        assert!(matches!(
            replay.pending,
            Some(ReplayCommand::Seek { position: 200, .. })
        ));
        replay.set_playing(true);
        assert!(matches!(
            replay.pending,
            Some(ReplayCommand::Seek { position: 200, .. })
        ));
        replay.seek(0);
        assert_eq!(replay.position(), Some(100));
        replay.error = Some("disconnected".into());
        replay.seek(150);
        assert_eq!(replay.position(), Some(100));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn seeks_bypass_poll_delay_and_keep_only_latest_pending_target() {
        let backend = RobotBackend::new(
            tokio::runtime::Handle::current(),
            None,
            "/scrub_test".into(),
        )
        .await
        .unwrap();
        let node = backend.node();
        let key = control_key("/scrub_test");
        let queryable = node.session().declare_queryable(key.clone()).await.unwrap();
        let (received, mut commands) = tokio::sync::mpsc::channel(8);
        let gate = std::sync::Arc::new(tokio::sync::Semaphore::new(0));
        let server_gate = gate.clone();
        let status = ReplayStatus {
            instance: "test".into(),
            recording: "test".into(),
            generation: 0,
            start: 0,
            end: 1000,
            position: 0,
            playing: false,
            sources: Default::default(),
        };
        let mut server_status = status.clone();
        let server = tokio::spawn(async move {
            while let Ok(query) = queryable.recv_async().await {
                let ReplayCommand::Seek { position, .. } =
                    serde_json::from_slice(&query.payload().unwrap().to_bytes()).unwrap()
                else {
                    panic!("expected seek")
                };
                received.send(position).await.unwrap();
                server_gate.acquire().await.unwrap().forget();
                server_status.position = position;
                server_status.generation += 1;
                query
                    .reply(&key, serde_json::to_vec(&server_status).unwrap())
                    .await
                    .unwrap();
            }
        });
        let context = Context::default();
        let mut replay = ReplaySession::default();
        replay.namespace = "/scrub_test".into();
        replay.status = Some(status);
        replay.next_request = Some(Instant::now() + Duration::from_secs(60));
        assert!(!replay.request_due(Instant::now()));
        replay.seek(100);
        replay.dispatch(&context, &backend);
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(2), commands.recv())
                .await
                .unwrap(),
            Some(100)
        );
        for target in [200, 300, 400] {
            replay.seek(target);
            replay.dispatch(&context, &backend);
        }
        assert!(
            commands.try_recv().is_err(),
            "do not flood the server or cancel a mutating request"
        );
        gate.add_permits(1);
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                replay.update(&context, &backend);
                if replay.request.is_none() {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(
            replay.position(),
            Some(400),
            "an older response must not move the thumb backward"
        );
        assert!(
            replay.request_due(Instant::now()),
            "new seek must not wait for the poll interval"
        );
        replay.dispatch(&context, &backend);
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(2), commands.recv())
                .await
                .unwrap(),
            Some(400)
        );
        gate.add_permits(1);
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                replay.update(&context, &backend);
                if !replay.is_seeking() {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(replay.status.as_ref().unwrap().position, 400);
        server.abort();
    }
}
