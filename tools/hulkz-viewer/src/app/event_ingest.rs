use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::protocol::{DiscoveryOp, WorkerEvent};

use super::{
    state::ViewerApp, workspace_panel::WorkspacePanel, workspace_panels::ParametersPanelStatus,
};

impl ViewerApp {
    pub fn drain_worker_events(&mut self) {
        let started = std::time::Instant::now();
        let mut processed_events = 0_usize;
        let mut processed_event_bytes = 0_usize;

        loop {
            if processed_events >= self.config.max_events_per_frame
                || processed_event_bytes >= self.config.max_event_bytes_per_frame
                || started.elapsed() >= self.config.max_event_ingest_time_per_frame
            {
                break;
            }

            match self.runtime.event_rx.try_recv() {
                Ok(envelope) => {
                    processed_events = processed_events.saturating_add(1);
                    processed_event_bytes =
                        processed_event_bytes.saturating_add(envelope.approx_bytes);
                    match envelope.event {
                        WorkerEvent::StreamHistoryBegin {
                            stream_id,
                            generation,
                        } => {
                            if let Some(state) = self.workspace.stream_states.get_mut(&stream_id) {
                                state.generation = generation;
                                state.history_loading = true;
                                state.history_total_records = 0;
                            }
                        }
                        WorkerEvent::StreamRecordsChunk {
                            stream_id,
                            generation,
                            records,
                            source: _source,
                        } => {
                            if self
                                .workspace
                                .stream_states
                                .get(&stream_id)
                                .is_some_and(|state| state.generation != generation)
                            {
                                continue;
                            }
                            debug!(
                                stream_id,
                                count = records.len(),
                                "received records from worker"
                            );
                            for record in &records {
                                self.insert_global_timestamp(record.timestamp_nanos);
                            }
                            self.append_lane_samples(stream_id, records.as_slice());

                            if let Some(state) = self.workspace.stream_states.get_mut(&stream_id) {
                                state.history_total_records =
                                    state.history_total_records.saturating_add(records.len());
                            }

                            if self.ui.follow_live {
                                if let Some(latest_record) = records.last().cloned() {
                                    let state =
                                        self.workspace.stream_states.entry(stream_id).or_default();
                                    state.current_record = Some(latest_record);
                                }
                                self.jump_latest_internal(false);
                            }
                        }
                        WorkerEvent::StreamHistoryEnd {
                            stream_id,
                            generation,
                            total_records,
                        } => {
                            if self
                                .workspace
                                .stream_states
                                .get(&stream_id)
                                .is_some_and(|state| state.generation != generation)
                            {
                                continue;
                            }
                            if let Some(state) = self.workspace.stream_states.get_mut(&stream_id) {
                                state.history_loading = false;
                                state.history_total_records = total_records;
                            }
                        }
                        WorkerEvent::SourceBound {
                            stream_id,
                            generation,
                            label,
                            binding,
                        } => {
                            info!(stream_id, %label, "worker bound source");
                            let state = self.workspace.stream_states.entry(stream_id).or_default();
                            state.generation = generation;
                            state.source_label = label;
                            state.current_record = None;
                            state.source_stats = None;
                            state.history_loading = true;
                            state.history_total_records = 0;
                            self.bind_stream_lane(stream_id, binding);
                            if let Some(anchor) = self.current_anchor_nanos() {
                                self.timeline.pending_scrub_anchor = Some(anchor);
                            }
                            self.ui.last_error = None;
                        }
                        WorkerEvent::AnchorRecord {
                            stream_id,
                            anchor_nanos,
                            record,
                        } => {
                            if self.current_anchor_nanos() == Some(anchor_nanos) {
                                let state =
                                    self.workspace.stream_states.entry(stream_id).or_default();
                                state.current_record = record;
                            }
                        }
                        WorkerEvent::DiscoveryPatch { op } => {
                            self.apply_discovery_patch(op);
                        }
                        WorkerEvent::DiscoverySnapshot {
                            publishers,
                            parameters,
                            sessions,
                        } => {
                            self.discovery.publishers = publishers;
                            self.discovery.parameters = parameters;
                            self.discovery.sessions = sessions;
                        }
                        WorkerEvent::ParameterValueLoaded {
                            target,
                            value_pretty,
                        } => {
                            for (_, tab) in self.workspace.dock_state.iter_all_tabs_mut() {
                                if let WorkspacePanel::Parameters(panel) = tab {
                                    if panel.selected_parameter_reference.as_ref() == Some(&target)
                                    {
                                        panel.editor_text = value_pretty.clone();
                                        panel.status = Some(ParametersPanelStatus {
                                            success: true,
                                            message: "Parameter loaded".to_string(),
                                        });
                                    }
                                }
                            }
                        }
                        WorkerEvent::ParameterWriteResult {
                            target,
                            success,
                            message,
                        } => {
                            for (_, tab) in self.workspace.dock_state.iter_all_tabs_mut() {
                                if let WorkspacePanel::Parameters(panel) = tab {
                                    if panel.selected_parameter_reference.as_ref() == Some(&target)
                                    {
                                        panel.status = Some(ParametersPanelStatus {
                                            success,
                                            message: message.clone(),
                                        });
                                    }
                                }
                            }
                        }
                        WorkerEvent::StreamStats { stream_id, source } => {
                            self.workspace
                                .stream_states
                                .entry(stream_id)
                                .or_default()
                                .source_stats = Some(*source);
                        }
                        WorkerEvent::BackendStats { backend } => {
                            self.ui.backend_stats = Some(*backend);
                        }
                        WorkerEvent::Error(message) => {
                            warn!(%message, "worker reported error");
                            self.ui.last_error = Some(message);
                        }
                        WorkerEvent::Ready => {
                            info!("worker is ready");
                            self.ui.ready = true;
                        }
                    }
                }
                Err(mpsc::error::TryRecvError::Empty) => break,
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.ui.last_error = Some("worker disconnected".to_string());
                    warn!("worker event channel disconnected");
                    break;
                }
            }
        }

        self.ui.frame_processed_events = processed_events;
        self.ui.frame_processed_event_bytes = processed_event_bytes;
        if self.runtime.event_rx.is_empty() {
            self.runtime
                .worker_wake_armed
                .store(false, std::sync::atomic::Ordering::SeqCst);
        }
    }

    fn apply_discovery_patch(&mut self, op: DiscoveryOp) {
        fn upsert_sorted<T: Ord>(items: &mut Vec<T>, item: T) {
            match items.binary_search(&item) {
                Ok(index) => items[index] = item,
                Err(index) => items.insert(index, item),
            }
        }

        fn remove_sorted<T: Ord>(items: &mut Vec<T>, item: &T) {
            if let Ok(index) = items.binary_search(item) {
                items.remove(index);
            }
        }

        match op {
            DiscoveryOp::PublisherUpsert(item) => {
                upsert_sorted(&mut self.discovery.publishers, item)
            }
            DiscoveryOp::PublisherRemove(item) => {
                remove_sorted(&mut self.discovery.publishers, &item)
            }
            DiscoveryOp::ParameterUpsert(item) => {
                upsert_sorted(&mut self.discovery.parameters, item)
            }
            DiscoveryOp::ParameterRemove(item) => {
                remove_sorted(&mut self.discovery.parameters, &item)
            }
            DiscoveryOp::SessionUpsert(item) => upsert_sorted(&mut self.discovery.sessions, item),
            DiscoveryOp::SessionRemove(item) => remove_sorted(&mut self.discovery.sessions, &item),
            DiscoveryOp::ResetNamespace(namespace) => {
                let _ = namespace;
                self.discovery.publishers.clear();
                self.discovery.parameters.clear();
                self.discovery.sessions.clear();
            }
        }
    }
}
