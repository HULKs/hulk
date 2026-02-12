use std::time::{Duration, Instant};

use color_eyre::{eyre::WrapErr as _, Result};
use eframe::{
    egui::{self, Color32},
    App, CreationContext, Frame,
};
use hulkz_stream::{BackendStats, SourceStats};
use tokio::{runtime::Runtime, sync::mpsc};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use crate::{
    model::{
        should_emit_scrub_command, RecordRow, ViewerConfig, ViewerState, WorkerCommand, WorkerEvent,
    },
    worker::run_worker,
};

pub struct ViewerApp {
    state: ViewerState,
    config: ViewerConfig,
    command_tx: mpsc::UnboundedSender<WorkerCommand>,
    event_rx: mpsc::UnboundedReceiver<WorkerEvent>,
    runtime: Runtime,
    worker_task: Option<tokio::task::JoinHandle<()>>,
    cancellation_token: CancellationToken,
    ingest_enabled: bool,
    last_error: Option<String>,
    source_stats: Option<SourceStats>,
    backend_stats: Option<BackendStats>,
    pending_scrub_anchor: Option<u64>,
    last_scrub_emitted: Instant,
    ready: bool,
    shutdown_started: bool,
}

impl ViewerApp {
    pub fn new(_creation_context: &CreationContext<'_>) -> Result<Self> {
        info!("starting hulkz-viewer app runtime");
        let runtime = Runtime::new().wrap_err("failed to create tokio runtime")?;

        let config = ViewerConfig::default();
        let state = ViewerState::new();
        let cancellation_token = CancellationToken::new();

        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let worker_task = runtime.spawn(run_worker(
            config.clone(),
            command_rx,
            event_tx,
            cancellation_token.clone(),
        ));
        info!("worker task spawned");

        let now = Instant::now();
        let last_scrub_emitted = now.checked_sub(config.scrub_debounce).unwrap_or(now);

        Ok(Self {
            state,
            config,
            command_tx,
            event_rx,
            runtime,
            worker_task: Some(worker_task),
            cancellation_token,
            ingest_enabled: true,
            last_error: None,
            source_stats: None,
            backend_stats: None,
            pending_scrub_anchor: None,
            last_scrub_emitted,
            ready: false,
            shutdown_started: false,
        })
    }

    fn send_command(&mut self, command: WorkerCommand) {
        debug!(?command, "sending worker command");
        if self.command_tx.send(command).is_err() {
            self.last_error = Some("worker command channel is closed".to_string());
            warn!("failed to send worker command: channel closed");
        }
    }

    fn drain_worker_events(&mut self) {
        loop {
            match self.event_rx.try_recv() {
                Ok(WorkerEvent::RecordsAppended(rows)) => {
                    debug!(count = rows.len(), "received records from worker");
                    self.state.append_records(rows);
                }
                Ok(WorkerEvent::Stats { source, backend }) => {
                    self.source_stats = Some(*source);
                    self.backend_stats = Some(*backend);
                }
                Ok(WorkerEvent::Error(message)) => {
                    warn!(%message, "worker reported error");
                    self.last_error = Some(message);
                }
                Ok(WorkerEvent::Ready) => {
                    info!("worker is ready");
                    self.ready = true;
                }
                Err(mpsc::error::TryRecvError::Empty) => break,
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.last_error = Some("worker disconnected".to_string());
                    warn!("worker event channel disconnected");
                    break;
                }
            }
        }
    }

    fn queue_scrub_for_current_selection(&mut self) {
        self.pending_scrub_anchor = self.state.selected().map(|record| record.timestamp_nanos);
    }

    fn maybe_emit_scrub_command(&mut self) {
        let Some(anchor) = self.pending_scrub_anchor else {
            return;
        };

        let now = Instant::now();
        if should_emit_scrub_command(self.last_scrub_emitted, now, self.config.scrub_debounce) {
            self.send_command(WorkerCommand::SetScrubAnchor(anchor));
            self.last_scrub_emitted = now;
            self.pending_scrub_anchor = None;
        }
    }

    fn draw_controls(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.checkbox(&mut self.ingest_enabled, "Ingest").changed() {
                self.send_command(WorkerCommand::SetIngestEnabled(self.ingest_enabled));
            }

            if ui
                .checkbox(&mut self.state.follow_live, "Follow live")
                .changed()
            {
                self.send_command(WorkerCommand::SetFollowLive(self.state.follow_live));
                if self.state.follow_live {
                    self.state.jump_latest();
                }
            }

            if ui.button("Prev").clicked() {
                self.state.follow_live = false;
                self.state.step_prev();
                self.queue_scrub_for_current_selection();
            }
            if ui.button("Next").clicked() {
                self.state.follow_live = false;
                self.state.step_next();
                self.queue_scrub_for_current_selection();
            }
            if ui.button("Jump Latest").clicked() {
                self.state.jump_latest();
                self.state.follow_live = true;
                self.queue_scrub_for_current_selection();
            }
        });

        if !self.state.records.is_empty() {
            let max_index = self.state.records.len().saturating_sub(1);
            let mut selected = self.state.selected_index.unwrap_or(max_index) as i32;
            if ui
                .add(egui::Slider::new(&mut selected, 0..=max_index as i32).text("Scrub"))
                .changed()
            {
                self.state.follow_live = false;
                self.state.selected_index = Some(selected as usize);
                self.queue_scrub_for_current_selection();
            }
        }
    }

    fn draw_record_list(&mut self, ui: &mut egui::Ui) {
        ui.heading("Records");
        ui.separator();

        let mut clicked_index: Option<usize> = None;
        egui::ScrollArea::vertical()
            .id_salt("record_list")
            .show(ui, |ui| {
                for (index, record) in self.state.records.iter().enumerate().rev() {
                    let selected = self.state.selected_index == Some(index);
                    let label = format_record_summary(record);
                    if ui.selectable_label(selected, label).clicked() {
                        clicked_index = Some(index);
                    }
                }
            });

        if let Some(index) = clicked_index {
            self.state.selected_index = Some(index);
            self.state.follow_live = false;
            self.queue_scrub_for_current_selection();
        }
    }

    fn draw_record_details(&self, ui: &mut egui::Ui) {
        ui.heading("Record");
        ui.separator();

        if let Some(record) = self.state.selected() {
            ui.label(format!(
                "Timestamp: {}",
                format_timestamp(record.timestamp_nanos)
            ));
            ui.label(format!(
                "Namespace: {}",
                record.effective_namespace.as_deref().unwrap_or("<none>")
            ));
            ui.separator();

            let mut body = record
                .json_pretty
                .clone()
                .or_else(|| record.raw_fallback.clone())
                .unwrap_or_else(|| "<empty payload>".to_string());

            ui.add(
                egui::TextEdit::multiline(&mut body)
                    .font(egui::TextStyle::Monospace)
                    .desired_rows(28)
                    .interactive(false),
            );
        } else {
            ui.label("No record selected yet");
        }
    }

    fn draw_status_bar(&self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label(format!("Ready: {}", self.ready));
            ui.separator();
            ui.label(format!("Records: {}", self.state.records.len()));

            if let Some(source_stats) = &self.source_stats {
                ui.separator();
                ui.label(format!("Durable len: {}", source_stats.durable_len));
            }

            if let Some(backend_stats) = &self.backend_stats {
                ui.separator();
                ui.label(format!("Active sources: {}", backend_stats.active_sources));
                ui.separator();
                ui.label(format!("Queue depth: {}", backend_stats.writer_queue_depth));
            }

            if let Some(error) = &self.last_error {
                ui.separator();
                ui.colored_label(Color32::RED, format!("Error: {error}"));
            }
        });
    }

    fn initiate_shutdown(&mut self) {
        if self.shutdown_started {
            return;
        }
        self.shutdown_started = true;
        info!("shutting down viewer app");

        self.send_command(WorkerCommand::Shutdown);
        self.cancellation_token.cancel();

        if let Some(worker_task) = self.worker_task.take() {
            let _ = self.runtime.block_on(async {
                tokio::time::timeout(Duration::from_secs(2), worker_task).await
            });
        }
        info!("viewer shutdown sequence completed");
    }
}

impl App for ViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        self.drain_worker_events();

        egui::TopBottomPanel::top("controls").show(ctx, |ui| {
            self.draw_controls(ui);
        });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            self.draw_status_bar(ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.state.records.is_empty() {
                ui.label("Waiting for JSON messages on demo/odometry (view plane)...");
            }

            ui.columns(2, |columns| {
                self.draw_record_list(&mut columns[0]);
                self.draw_record_details(&mut columns[1]);
            });
        });

        self.maybe_emit_scrub_command();
        ctx.request_repaint_after(Duration::from_millis(50));
    }
}

impl Drop for ViewerApp {
    fn drop(&mut self) {
        self.initiate_shutdown();
    }
}

fn format_timestamp(nanos: u64) -> String {
    format!("{:.3}s ({nanos} ns)", nanos as f64 / 1_000_000_000.0)
}

fn format_record_summary(record: &RecordRow) -> String {
    let namespace = record.effective_namespace.as_deref().unwrap_or("<none>");
    format!(
        "{} | {}",
        format_timestamp(record.timestamp_nanos),
        namespace
    )
}
