//! Client-local playback. The Timeline panel retains its presentation and controls.
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use color_eyre::eyre::{Result, eyre};
use eframe::egui::Context;
use ros_z::time::Time;
use ros_z_debug::query::{Cursor, CursorHandle, RemoteMcap};

use tokio::{
    sync::{
        oneshot::{self, error::TryRecvError},
        watch,
    },
    task::JoinHandle,
    time::MissedTickBehavior,
};

use crate::backend::RobotBackend;

const SCRUB_INTERVAL: Duration = Duration::from_millis(125);

#[derive(Clone, Debug)]
pub struct ReplayStatus {
    pub instance: String,
    pub recording: String,
    pub start: u64,
    pub end: u64,
    pub position: u64,
    pub playing: bool,
}

#[derive(Default)]
pub struct ReplaySession {
    namespace: String,
    pub status: Option<ReplayStatus>,
    pub error: Option<String>,
    request: Option<(JoinHandle<()>, oneshot::Receiver<Result<RemoteMcap>>)>,
    retry_at: Option<Instant>,
    cursor: Option<CursorHandle>,
    remote: Option<Arc<RemoteMcap>>,
    playback: Option<watch::Sender<(u64, bool, Instant)>>,
    scrub_preview_at: Option<Instant>,
}

impl ReplaySession {
    pub fn position(&self) -> Option<u64> {
        self.status.as_ref().map(|status| status.position)
    }

    pub fn seek(&mut self, position: u64) {
        self.scrub_preview_at = None;
        let Some(status) = &mut self.status else {
            return;
        };
        status.position = position.clamp(status.start, status.end);
        status.playing = false;
        if let Some(playback) = &self.playback {
            playback.send_replace((status.position, false, Instant::now()));
        }
        if let Some(cursor) = &self.cursor {
            cursor.set(Cursor::At(Time::from_nanos(status.position as i64)));
        }
    }

    pub fn scrub_to(&mut self, position: u64) {
        self.scrub_to_at(position, Instant::now());
    }

    fn scrub_to_at(&mut self, position: u64, now: Instant) {
        let Some(status) = &mut self.status else {
            return;
        };
        status.position = position.clamp(status.start, status.end);
        if self
            .scrub_preview_at
            .is_none_or(|last| now.duration_since(last) >= SCRUB_INTERVAL)
        {
            self.seek(position);
            self.scrub_preview_at = Some(now);
        }
    }

    pub fn finish_scrub(&mut self) {
        if self.scrub_preview_at.take().is_some()
            && let Some(position) = self.position()
        {
            self.seek(position);
        }
    }

    pub fn set_playing(&mut self, playing: bool) {
        self.finish_scrub();
        let Some(status) = &mut self.status else {
            return;
        };
        status.playing = playing && status.position < status.end;
        if let Some(playback) = &self.playback {
            playback.send_replace((status.position, status.playing, Instant::now()));
        }
    }

    pub fn retry(&mut self) {
        self.retry_at = None;
        self.status = None;
        self.cursor = None;
        self.remote = None;
        self.playback = None;
        self.scrub_preview_at = None;
    }

    pub fn update(&mut self, context: &Context, backend: &RobotBackend) {
        if self
            .remote
            .as_ref()
            .is_some_and(|remote| remote.needs_reconnect())
        {
            self.retry();
        }
        let namespace = backend.namespace();
        if self.namespace != namespace {
            backend.use_live();
            *self = Self::default();
            self.namespace = namespace;
        }
        let completed = self
            .request
            .as_mut()
            .and_then(|(_, receiver)| match receiver.try_recv() {
                Ok(result) => Some(result),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Closed) => Some(Err(eyre!("recording discovery stopped"))),
            });
        if let Some(result) = completed {
            self.request = None;
            match result {
                Ok(remote) => {
                    let remote = Arc::new(remote);
                    let info = remote.recording();
                    let start = info.start.as_nanos() as u64;
                    let end = info.end.as_nanos() as u64;
                    self.status = Some(ReplayStatus {
                        instance: info.instance.clone(),
                        recording: info.name.clone(),
                        start,
                        end,
                        position: start,
                        playing: false,
                    });
                    let cursor = CursorHandle::new(Cursor::At(info.start));
                    let (playback, mut commands) = watch::channel((start, false, Instant::now()));
                    let driver_cursor = cursor.clone();
                    let driver_remote = Arc::clone(&remote);
                    backend.runtime_handle().spawn(async move {
                        let mut tick = tokio::time::interval(Duration::from_millis(16));
                        tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
                        loop {
                            let playing = {
                                let command = commands.borrow_and_update();
                                let (origin, playing, started) = *command;
                                if playing {
                                    let end = driver_remote.bounds().1.as_nanos() as u64;
                                    let position = origin
                                        .saturating_add(
                                            started.elapsed().as_nanos().min(u64::MAX as u128)
                                                as u64,
                                        )
                                        .min(end);
                                    driver_cursor.advance(Time::from_nanos(position as i64));
                                }
                                playing
                            };
                            tokio::select! {
                                changed = commands.changed() => if changed.is_err() { break; },
                                _ = tick.tick(), if playing => {},
                            }
                        }
                    });
                    backend.use_recording(Arc::clone(&remote), cursor.clone());
                    self.remote = Some(remote);
                    self.cursor = Some(cursor);
                    self.playback = Some(playback);
                    self.error = None;
                }
                Err(error) => {
                    self.error = Some(format!("{error:#}"));
                    self.retry_at = Some(Instant::now() + Duration::from_secs(2));
                }
            }
        }
        if let (Some(status), Some(cursor)) = (&mut self.status, &self.cursor) {
            if let Some(remote) = &self.remote {
                let (start, end) = remote.bounds();
                status.start = start.as_nanos() as u64;
                status.end = end.as_nanos() as u64;
            }
            if self.scrub_preview_at.is_none()
                && let Cursor::At(time) = cursor.position().cursor
            {
                status.position = time.as_nanos() as u64;
                if status.position >= status.end && status.playing {
                    status.playing = false;
                    if let Some(playback) = &self.playback {
                        playback.send_replace((status.end, false, Instant::now()));
                    }
                }
            }
            if status.playing {
                context.request_repaint_after(Duration::from_millis(16));
            }
        } else {
            context.request_repaint_after(Duration::from_secs(1));
        }
    }

    pub fn dispatch(&mut self, context: &Context, backend: &RobotBackend) {
        // Finish even if the pointer was released outside the timeline or its tab was closed.
        if self.scrub_preview_at.is_some() {
            if context.input(|input| input.focused && input.pointer.primary_down()) {
                if let Some(position) = self.position() {
                    self.scrub_to(position);
                }
                if let Some(last) = self.scrub_preview_at {
                    context.request_repaint_after(SCRUB_INTERVAL.saturating_sub(last.elapsed()));
                }
            } else {
                self.finish_scrub();
            }
        }
        if self.status.is_none()
            && self.request.is_none()
            && self.retry_at.is_none_or(|time| Instant::now() >= time)
        {
            let node = backend.node();
            let namespace = self.namespace.clone();
            let context = context.clone();
            let (sender, receiver) = oneshot::channel();
            let task = backend.runtime_handle().spawn(async move {
                let result = tokio::time::timeout(
                    Duration::from_secs(3),
                    RemoteMcap::connect(node, &namespace),
                )
                .await
                .map_err(|_| eyre!("recording server unavailable"))
                .and_then(|result| result.map_err(Into::into));
                let _ = sender.send(result);
                context.request_repaint();
            });
            self.request = Some((task, receiver));
        }
    }
}

impl Drop for ReplaySession {
    fn drop(&mut self) {
        if let Some((task, _)) = &self.request {
            task.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::runtime::Handle;

    use super::*;

    fn session() -> ReplaySession {
        let mut replay = ReplaySession::default();
        replay.status = Some(ReplayStatus {
            instance: "test".into(),
            recording: "test".into(),
            start: 10,
            end: 20,
            position: 10,
            playing: false,
        });
        replay.cursor = Some(CursorHandle::new(Cursor::At(Time::from_nanos(10))));
        replay
    }

    #[test]
    fn local_seeks_clamp_and_pause_without_network() {
        let mut replay = session();
        replay.set_playing(true);
        replay.seek(100);
        assert_eq!(replay.position(), Some(20));
        assert!(!replay.status.as_ref().unwrap().playing);
        assert_eq!(
            replay.cursor.as_ref().unwrap().position().cursor,
            Cursor::At(Time::from_nanos(20))
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn namespace_change_after_retry_restores_live_connection() {
        let backend = RobotBackend::new(Handle::current(), None, "/recording".into())
            .await
            .unwrap();
        let mut replay = session();
        replay.namespace = backend.namespace();
        backend
            .query_connection()
            .cursor
            .set(Cursor::At(Time::from_nanos(10)));
        let previous = backend.presentation_identity();
        replay.retry();
        backend.set_namespace("/robot".into()).unwrap();
        replay.update(&Context::default(), &backend);
        assert_eq!(
            backend.query_connection().cursor.position().cursor,
            Cursor::Live
        );
        assert_ne!(backend.presentation_identity().0, previous.0);
        assert_eq!(replay.namespace, "/robot");
    }

    #[test]
    fn same_position_seek_preserves_cursor_revision() {
        let mut replay = session();
        let cursor = replay.cursor.as_ref().unwrap().clone();
        replay.seek(15);
        let revision = cursor.position().revision;
        replay.set_playing(true);
        replay.seek(15);
        assert_eq!(cursor.position().revision, revision);
        assert!(!replay.status.as_ref().unwrap().playing);
    }

    #[test]
    fn scrubbing_coalesces_at_eight_hz_and_release_flushes_immediately() {
        let mut replay = session();
        replay.status.as_mut().unwrap().end = 1_000;
        replay.set_playing(true);
        let cursor = replay.cursor.as_ref().unwrap().clone();
        let now = Instant::now();
        replay.scrub_to_at(11, now);
        assert!(!replay.status.as_ref().unwrap().playing);
        let revision = cursor.position().revision;
        for millis in 1..125 {
            replay.scrub_to_at(11 + millis, now + Duration::from_millis(millis));
            assert_eq!(
                replay.position(),
                Some(11 + millis),
                "playhead follows every move"
            );
            assert_eq!(
                cursor.position().revision,
                revision,
                "pending moves must not invalidate previews"
            );
        }
        assert_eq!(cursor.position().cursor, Cursor::At(Time::from_nanos(11)));
        replay.scrub_to_at(200, now + SCRUB_INTERVAL);
        assert_eq!(cursor.position().cursor, Cursor::At(Time::from_nanos(200)));
        assert_eq!(cursor.position().revision, revision + 1);
        replay.scrub_to_at(250, now + SCRUB_INTERVAL + Duration::from_millis(1));
        replay.finish_scrub();
        assert_eq!(cursor.position().cursor, Cursor::At(Time::from_nanos(250)));
        assert!(replay.scrub_preview_at.is_none());
        replay.finish_scrub();
        assert_eq!(
            cursor.position().revision,
            revision + 2,
            "release is idempotent"
        );

        replay.scrub_to_at(300, now);
        replay.scrub_to_at(350, now + Duration::from_millis(1));
        replay.seek(400);
        replay.finish_scrub();
        assert_eq!(
            cursor.position().cursor,
            Cursor::At(Time::from_nanos(400)),
            "explicit seeks supersede pending scrubs"
        );
        replay.scrub_to_at(500, now);
        replay.scrub_to_at(550, now + Duration::from_millis(1));
        replay.set_playing(true);
        assert_eq!(cursor.position().cursor, Cursor::At(Time::from_nanos(550)));
        assert!(replay.status.as_ref().unwrap().playing);
        replay.scrub_to_at(600, now);
        replay.retry();
        assert!(replay.scrub_preview_at.is_none());
    }
}
