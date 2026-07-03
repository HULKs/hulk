use std::{collections::BTreeSet, future::Future, time::Duration};

use color_eyre::{
    Report,
    eyre::{Context as _, eyre},
};
use eframe::egui::{ScrollArea, TextEdit, Ui};
use hulk_widgets::CompletionEdit;
use ros_z::parameter::{
    GetNodeParameterValueResponse, GetNodeParametersSnapshotResponse, NodeParameterEvent,
    RemoteParameterClient, SetNodeParameterResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use crate::panel::{Panel, PanelCreationContext, PanelUiContext};

const PARAMETER_SNAPSHOT_SUFFIX: &str = "/parameter/get_snapshot";
const REMOTE_PARAMETER_TIMEOUT: Duration = Duration::from_secs(3);

pub struct ParameterPanel {
    node_editor: String,
    node: String,
    path_editor: String,
    path: String,
    layer_editor: String,
    layer: String,
    snapshot: Option<SnapshotState>,
    value_revision: Option<u64>,
    clean_value_editor: String,
    latest_snapshot_revision: Option<u64>,
    effective_source_layer: String,
    value_editor: String,
    status: Status,
    remote: RemoteState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SnapshotState {
    parameter_key: String,
    revision: u64,
    layers: Vec<String>,
    path_completions: Vec<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
enum Status {
    #[default]
    Idle,
    Info(String),
    Error(String),
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct SavedState {
    #[serde(default)]
    node: String,
    #[serde(default)]
    path: String,
    #[serde(default)]
    layer: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SnapshotRequest {
    node: String,
    kind: SnapshotRequestKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum SnapshotRequestKind {
    Auto,
    Manual,
    AfterSet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ValueRequest {
    node: String,
    path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SetRequest {
    node: String,
    path: String,
    target_layer: String,
    expected_revision: u64,
}

#[derive(Debug)]
enum RemoteOperationResult {
    Snapshot {
        request: SnapshotRequest,
        result: Result<GetNodeParametersSnapshotResponse, String>,
    },
    SnapshotMetadata {
        request: SnapshotRequest,
        result: Result<GetNodeParametersSnapshotResponse, String>,
    },
    Value {
        request: ValueRequest,
        result: Result<GetNodeParameterValueResponse, String>,
    },
    Set {
        request: SetRequest,
        result: Result<SetNodeParameterResponse, String>,
    },
    Event {
        generation: u64,
        node: String,
        revision: u64,
    },
    EventSubscriptionFailed {
        generation: u64,
        node: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RemoteFollowUp {
    None,
    FetchValue,
    RefreshAfterSet,
    RefreshSnapshot,
    RefreshSnapshotMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NodeRemoteAction {
    Current,
    Changed,
    Invalid,
}

struct RemoteState {
    sender: UnboundedSender<RemoteOperationResult>,
    receiver: UnboundedReceiver<RemoteOperationResult>,
    pending_count: usize,
    event_subscription_node: Option<String>,
    event_subscription_generation: u64,
    refresh_snapshot_after_pending: bool,
    refresh_snapshot_metadata_after_pending: bool,
}

impl Default for RemoteState {
    fn default() -> Self {
        let (sender, receiver) = unbounded_channel();
        Self {
            sender,
            receiver,
            pending_count: 0,
            event_subscription_node: None,
            event_subscription_generation: 0,
            refresh_snapshot_after_pending: false,
            refresh_snapshot_metadata_after_pending: false,
        }
    }
}

impl Default for ParameterPanel {
    fn default() -> Self {
        Self {
            node_editor: String::new(),
            node: String::new(),
            path_editor: String::new(),
            path: String::new(),
            layer_editor: String::new(),
            layer: String::new(),
            snapshot: None,
            value_revision: None,
            clean_value_editor: String::new(),
            latest_snapshot_revision: None,
            effective_source_layer: String::new(),
            value_editor: String::new(),
            status: Status::Idle,
            remote: RemoteState::default(),
        }
    }
}

impl Panel for ParameterPanel {
    const STORAGE_ID: &'static str = "parameter";
    const DISPLAY_NAME: &'static str = "Parameter";

    fn new(context: PanelCreationContext<'_>) -> Self {
        let saved = context
            .value
            .and_then(|value| serde_json::from_value::<SavedState>(value.clone()).ok())
            .unwrap_or_default();

        Self {
            node_editor: saved.node.clone(),
            node: saved.node,
            path_editor: saved.path.clone(),
            path: saved.path,
            layer_editor: saved.layer.clone(),
            layer: saved.layer,
            ..Default::default()
        }
    }

    fn ui(&mut self, ui: &mut Ui, context: PanelUiContext<'_>) {
        self.drain_remote_results(&context);
        self.ensure_event_subscription(&context);

        let namespace = context.backend.namespace();
        let service_names = {
            let graph = context.backend.graph().lock();
            graph
                .services()
                .map(|service| service.topic.clone())
                .collect::<Vec<_>>()
        };
        let nodes = parameter_node_completions(&service_names, &namespace, &self.node_editor);
        let (layers, path_completions) = self
            .snapshot
            .as_ref()
            .map(|snapshot| (snapshot.layers.clone(), snapshot.path_completions.clone()))
            .unwrap_or_default();

        ui.vertical(|ui| {
            ui.add_enabled_ui(self.can_edit_selectors(), |ui| {
                ui.horizontal(|ui| {
                    ui.label("Node");
                    let node_response = ui.add(CompletionEdit::new(
                        ui.id().with("parameter_node"),
                        &nodes,
                        &mut self.node_editor,
                    ));
                    if (node_response.changed() || node_response.lost_focus())
                        && self.commit_node(&service_names, &namespace)
                    {
                        self.spawn_snapshot_fetch(&context, SnapshotRequestKind::Auto);
                    }

                    ui.label("Path");
                    let path_response = ui.add(CompletionEdit::new(
                        ui.id().with("parameter_path"),
                        &path_completions,
                        &mut self.path_editor,
                    ));
                    if (path_response.changed() || path_response.lost_focus()) && self.commit_path()
                    {
                        self.spawn_value_fetch(&context);
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Layer");
                    let layer_response = ui.add(CompletionEdit::new(
                        ui.id().with("parameter_layer"),
                        &layers,
                        &mut self.layer_editor,
                    ));
                    if layer_response.changed() || layer_response.lost_focus() {
                        self.commit_layer();
                    }
                });
            });

            ui.horizontal(|ui| {
                ui.add_enabled_ui(self.can_start_remote_operation(), |ui| {
                    if ui.button("Refresh").clicked() {
                        match self.validate_node_for_remote_action(&service_names, &namespace) {
                            NodeRemoteAction::Current | NodeRemoteAction::Changed => {
                                self.commit_path();
                                self.commit_layer();
                                self.spawn_snapshot_fetch(&context, SnapshotRequestKind::Manual);
                            }
                            NodeRemoteAction::Invalid => {}
                        }
                    }
                });

                let can_set = self.can_start_remote_operation()
                    && !self.node_editor.trim().is_empty()
                    && !self.path_editor.trim().is_empty()
                    && !self.layer_editor.trim().is_empty()
                    && !self.value_editor.trim().is_empty();
                ui.add_enabled_ui(can_set, |ui| {
                    if ui.button("Set").clicked() {
                        match self.validate_node_for_remote_action(&service_names, &namespace) {
                            NodeRemoteAction::Current => self.spawn_set_value(&context),
                            NodeRemoteAction::Changed => {
                                self.spawn_snapshot_fetch(&context, SnapshotRequestKind::Auto)
                            }
                            NodeRemoteAction::Invalid => {}
                        }
                    }
                });
            });

            if nodes.is_empty() {
                ui.label("No parameter-capable nodes visible yet.");
            }

            self.render_metadata(ui);
            self.render_status(ui);
            ui.separator();

            ScrollArea::vertical().show(ui, |ui| {
                ui.add_enabled_ui(self.can_edit_value(), |ui| {
                    ui.add(
                        TextEdit::multiline(&mut self.value_editor)
                            .font(eframe::egui::TextStyle::Monospace)
                            .desired_width(f32::INFINITY)
                            .code_editor(),
                    );
                });
            });
        });
    }

    fn save(&self) -> Value {
        json!({
            "node": self.node,
            "path": self.path,
            "layer": self.layer,
        })
    }
}

impl ParameterPanel {
    fn commit_node(&mut self, service_names: &[String], namespace: &str) -> bool {
        let next = match resolve_parameter_node(service_names, namespace, &self.node_editor) {
            Ok(node) => node,
            Err(error) => {
                self.status = Status::Error(format!("{error:#}"));
                return false;
            }
        };
        if next == self.node {
            return false;
        }
        self.apply_node_change(next);
        true
    }

    fn validate_node_for_remote_action(
        &mut self,
        service_names: &[String],
        namespace: &str,
    ) -> NodeRemoteAction {
        let next = match resolve_parameter_node(service_names, namespace, &self.node_editor) {
            Ok(node) => node,
            Err(error) => {
                self.status = Status::Error(format!("{error:#}"));
                return NodeRemoteAction::Invalid;
            }
        };

        if next == self.node {
            NodeRemoteAction::Current
        } else {
            self.apply_node_change(next);
            NodeRemoteAction::Changed
        }
    }

    fn apply_node_change(&mut self, next: String) {
        self.node = next;
        self.snapshot = None;
        self.path_editor.clear();
        self.path.clear();
        self.clear_value_state();
        self.remote.event_subscription_node = None;
        self.remote.event_subscription_generation += 1;
        self.remote.refresh_snapshot_after_pending = false;
        self.remote.refresh_snapshot_metadata_after_pending = false;
        self.status = Status::Idle;
    }

    fn commit_path(&mut self) -> bool {
        let next = self.path_editor.trim().to_string();
        if next == self.path {
            return false;
        }
        self.path = next;
        self.clear_value_state();
        self.status = Status::Idle;
        true
    }

    fn commit_layer(&mut self) {
        self.layer = self.layer_editor.trim().to_string();
    }

    fn can_edit_selectors(&self) -> bool {
        self.can_start_remote_operation()
    }

    fn can_edit_value(&self) -> bool {
        self.can_start_remote_operation() && !self.path.is_empty()
    }

    fn can_start_remote_operation(&self) -> bool {
        self.remote.pending_count == 0
    }

    fn drain_remote_results(&mut self, context: &PanelUiContext<'_>) {
        while let Ok(result) = self.remote.receiver.try_recv() {
            if !matches!(
                result,
                RemoteOperationResult::Event { .. }
                    | RemoteOperationResult::EventSubscriptionFailed { .. }
            ) {
                self.remote.pending_count = self.remote.pending_count.saturating_sub(1);
            }
            match self.handle_remote_result(result) {
                RemoteFollowUp::None => {}
                RemoteFollowUp::FetchValue => self.spawn_value_fetch(context),
                RemoteFollowUp::RefreshAfterSet => {
                    self.spawn_snapshot_fetch(context, SnapshotRequestKind::AfterSet)
                }
                RemoteFollowUp::RefreshSnapshot => {
                    self.spawn_snapshot_fetch(context, SnapshotRequestKind::Manual)
                }
                RemoteFollowUp::RefreshSnapshotMetadata => {
                    self.spawn_snapshot_metadata_fetch(context)
                }
            }
        }

        if self.remote.pending_count == 0 {
            if self.remote.refresh_snapshot_after_pending {
                self.remote.refresh_snapshot_after_pending = false;
                self.spawn_snapshot_fetch(context, SnapshotRequestKind::Manual);
            } else if self.remote.refresh_snapshot_metadata_after_pending {
                self.remote.refresh_snapshot_metadata_after_pending = false;
                self.spawn_snapshot_metadata_fetch(context);
            }
        }
    }

    fn spawn_snapshot_fetch(&mut self, context: &PanelUiContext<'_>, kind: SnapshotRequestKind) {
        if self.node.is_empty() {
            self.status = Status::Error("select a parameter node".to_string());
            return;
        }

        let request = SnapshotRequest {
            node: self.node.clone(),
            kind,
        };
        let backend_node = context.backend.node();
        let sender = self.remote.sender.clone();
        let egui_context = context.egui_context.clone();
        self.remote.pending_count += 1;
        self.status = Status::Info("Loading parameter snapshot...".to_string());

        context.backend.runtime_handle().spawn(async move {
            let target_node = request.node.clone();
            let result = run_remote_operation("fetching parameter snapshot", async move {
                let client = RemoteParameterClient::new(backend_node, target_node)
                    .wrap_err("failed to create remote parameter client")?;
                client
                    .get_snapshot()
                    .await
                    .wrap_err("failed to fetch parameter snapshot")
            })
            .await;
            let _ = sender.send(RemoteOperationResult::Snapshot { request, result });
            egui_context.request_repaint();
        });
    }

    fn spawn_snapshot_metadata_fetch(&mut self, context: &PanelUiContext<'_>) {
        if self.node.is_empty() {
            return;
        }
        let request = SnapshotRequest {
            node: self.node.clone(),
            kind: SnapshotRequestKind::Manual,
        };
        let backend_node = context.backend.node();
        let sender = self.remote.sender.clone();
        let egui_context = context.egui_context.clone();
        self.remote.pending_count += 1;

        context.backend.runtime_handle().spawn(async move {
            let target_node = request.node.clone();
            let result = run_remote_operation("fetching parameter snapshot metadata", async move {
                let client = RemoteParameterClient::new(backend_node, target_node)
                    .wrap_err("failed to create remote parameter client")?;
                client
                    .get_snapshot()
                    .await
                    .wrap_err("failed to fetch parameter snapshot")
            })
            .await;
            let _ = sender.send(RemoteOperationResult::SnapshotMetadata { request, result });
            egui_context.request_repaint();
        });
    }

    fn ensure_event_subscription(&mut self, context: &PanelUiContext<'_>) {
        if self.node.is_empty()
            || self.remote.event_subscription_node.as_deref() == Some(self.node.as_str())
        {
            return;
        }
        self.remote.event_subscription_node = Some(self.node.clone());
        self.remote.event_subscription_generation += 1;
        let generation = self.remote.event_subscription_generation;
        let node = self.node.clone();
        let backend_node = context.backend.node();
        let sender = self.remote.sender.clone();
        let egui_context = context.egui_context.clone();

        context.backend.runtime_handle().spawn(async move {
            let Ok(client) = RemoteParameterClient::new(backend_node, node.clone()) else {
                let _ = sender
                    .send(RemoteOperationResult::EventSubscriptionFailed { generation, node });
                egui_context.request_repaint();
                return;
            };
            let Ok(subscriber) = client.subscribe_events().await else {
                let _ = sender
                    .send(RemoteOperationResult::EventSubscriptionFailed { generation, node });
                egui_context.request_repaint();
                return;
            };
            while let Ok(event) = subscriber.recv().await {
                let NodeParameterEvent {
                    node_fqn, revision, ..
                } = event;
                let _ = sender.send(RemoteOperationResult::Event {
                    generation,
                    node: node_fqn,
                    revision,
                });
                egui_context.request_repaint();
            }
            let _ =
                sender.send(RemoteOperationResult::EventSubscriptionFailed { generation, node });
            egui_context.request_repaint();
        });
    }

    fn spawn_value_fetch(&mut self, context: &PanelUiContext<'_>) {
        if self.node.is_empty() || self.path.is_empty() {
            return;
        }

        let request = ValueRequest {
            node: self.node.clone(),
            path: self.path.clone(),
        };
        let backend_node = context.backend.node();
        let sender = self.remote.sender.clone();
        let egui_context = context.egui_context.clone();
        self.remote.pending_count += 1;
        self.status = Status::Info("Loading parameter value...".to_string());

        context.backend.runtime_handle().spawn(async move {
            let target_node = request.node.clone();
            let path = request.path.clone();
            let result = run_remote_operation("fetching parameter value", async move {
                let client = RemoteParameterClient::new(backend_node, target_node)
                    .wrap_err("failed to create remote parameter client")?;
                client
                    .get_value(path)
                    .await
                    .wrap_err("failed to fetch parameter value")
            })
            .await;
            let _ = sender.send(RemoteOperationResult::Value { request, result });
            egui_context.request_repaint();
        });
    }

    fn apply_snapshot_response(
        &mut self,
        response: GetNodeParametersSnapshotResponse,
    ) -> Result<(), Report> {
        if !response.success {
            return Err(eyre!("{}", response.message));
        }

        let path_completions = parse_snapshot_path_completions(&response.value_json)?;
        let rendered_snapshot = render_json_for_editor(&response.value_json)?;
        let default_layer = response.layers.last().cloned();
        let selected_layer = self.layer.clone();
        let selected_layer_missing =
            !selected_layer.is_empty() && !response.layers.contains(&selected_layer);
        let revision = response.revision;

        self.snapshot = Some(SnapshotState {
            parameter_key: response.parameter_key,
            revision,
            layers: response.layers,
            path_completions,
        });

        if selected_layer_missing {
            self.clear_value_state();
            return Err(eyre!(
                "selected target layer '{selected_layer}' is not active; choose an active layer before setting"
            ));
        }

        if self.layer.is_empty()
            && let Some(layer) = default_layer
        {
            self.layer = layer.clone();
            self.layer_editor = layer;
        }

        self.latest_snapshot_revision = Some(revision);
        if self.path.is_empty() {
            self.value_editor = rendered_snapshot;
            self.clean_value_editor = self.value_editor.clone();
            self.value_revision = Some(revision);
            self.effective_source_layer.clear();
        }

        self.status = Status::Info("Fetched parameter snapshot.".to_string());
        Ok(())
    }

    fn apply_value_response(
        &mut self,
        response: GetNodeParameterValueResponse,
    ) -> Result<(), Report> {
        if !response.success {
            return Err(eyre!("{}", response.message));
        }

        self.value_editor = render_json_for_editor(&response.value_json)?;
        self.clean_value_editor = self.value_editor.clone();
        self.value_revision = Some(response.revision);
        self.latest_snapshot_revision = Some(response.revision);
        self.effective_source_layer = response.effective_source_layer;
        self.status = Status::Info("Fetched parameter value.".to_string());
        Ok(())
    }

    fn spawn_set_value(&mut self, context: &PanelUiContext<'_>) {
        self.commit_path();
        self.commit_layer();

        let request = match self.set_request() {
            Ok(request) => request,
            Err(error) => {
                self.status = Status::Error(format!("{error:#}"));
                return;
            }
        };
        let value = match parse_editor_json(&self.value_editor) {
            Ok(value) => value,
            Err(error) => {
                self.status = Status::Error(format!("{error:#}"));
                return;
            }
        };

        let backend_node = context.backend.node();
        let sender = self.remote.sender.clone();
        let egui_context = context.egui_context.clone();
        self.remote.pending_count += 1;
        self.status = Status::Info("Setting parameter value...".to_string());

        context.backend.runtime_handle().spawn(async move {
            let target_node = request.node.clone();
            let path = request.path.clone();
            let target_layer = request.target_layer.clone();
            let expected_revision = request.expected_revision;
            let result = run_remote_operation("setting parameter value", async move {
                let client = RemoteParameterClient::new(backend_node, target_node)
                    .wrap_err("failed to create remote parameter client")?;
                client
                    .set_json(path, &value, target_layer, Some(expected_revision))
                    .await
                    .wrap_err("failed to set parameter value")
            })
            .await;
            let _ = sender.send(RemoteOperationResult::Set { request, result });
            egui_context.request_repaint();
        });
    }

    fn set_request(&self) -> Result<SetRequest, Report> {
        if self.path.is_empty() {
            return Err(eyre!("enter a parameter path"));
        }
        if self.layer.is_empty() {
            return Err(eyre!("select a target layer"));
        }
        if let Some(snapshot) = &self.snapshot
            && !snapshot.layers.contains(&self.layer)
        {
            return Err(eyre!(
                "selected target layer '{}' is not active; choose an active layer before setting",
                self.layer
            ));
        }
        if self.is_value_dirty() && self.is_stale_against_latest_revision() {
            return Err(eyre!(
                "parameter changed externally; refresh before setting to discard local edits"
            ));
        }

        Ok(SetRequest {
            node: self.node.clone(),
            path: self.path.clone(),
            target_layer: self.layer.clone(),
            expected_revision: self.expected_revision_for_set()?,
        })
    }

    fn expected_revision_for_set(&self) -> Result<u64, Report> {
        self.value_revision
            .or_else(|| self.snapshot.as_ref().map(|snapshot| snapshot.revision))
            .ok_or_else(|| eyre!("refresh the parameter snapshot or value before setting"))
    }

    fn handle_remote_result(&mut self, result: RemoteOperationResult) -> RemoteFollowUp {
        match result {
            RemoteOperationResult::Snapshot { request, result } => {
                self.handle_snapshot_result(request, result)
            }
            RemoteOperationResult::SnapshotMetadata { request, result } => {
                self.handle_snapshot_metadata_result(request, result)
            }
            RemoteOperationResult::Value { request, result } => {
                self.handle_value_result(request, result)
            }
            RemoteOperationResult::Set { request, result } => {
                self.handle_set_result(request, result)
            }
            RemoteOperationResult::Event {
                generation,
                node,
                revision,
            } => self.handle_event_result(generation, node, revision),
            RemoteOperationResult::EventSubscriptionFailed { generation, node } => {
                self.handle_event_subscription_failed(generation, node)
            }
        }
    }

    fn handle_snapshot_result(
        &mut self,
        request: SnapshotRequest,
        result: Result<GetNodeParametersSnapshotResponse, String>,
    ) -> RemoteFollowUp {
        if request.node != self.node {
            return RemoteFollowUp::None;
        }

        match result {
            Ok(response) => match self.apply_snapshot_response(response) {
                Ok(()) => {
                    if matches!(
                        request.kind,
                        SnapshotRequestKind::Manual | SnapshotRequestKind::AfterSet
                    ) && !self.path.is_empty()
                    {
                        RemoteFollowUp::FetchValue
                    } else {
                        RemoteFollowUp::None
                    }
                }
                Err(error) => {
                    self.status = Status::Error(format!("{error:#}"));
                    RemoteFollowUp::None
                }
            },
            Err(message) => {
                if request.kind == SnapshotRequestKind::Auto {
                    self.snapshot = None;
                }
                self.status = Status::Error(message);
                RemoteFollowUp::None
            }
        }
    }

    fn handle_snapshot_metadata_result(
        &mut self,
        request: SnapshotRequest,
        result: Result<GetNodeParametersSnapshotResponse, String>,
    ) -> RemoteFollowUp {
        if request.node != self.node {
            return RemoteFollowUp::None;
        }
        let Ok(response) = result else {
            return RemoteFollowUp::None;
        };
        let Ok(path_completions) = parse_snapshot_path_completions(&response.value_json) else {
            return RemoteFollowUp::None;
        };
        self.latest_snapshot_revision = Some(response.revision);
        self.snapshot = Some(SnapshotState {
            parameter_key: response.parameter_key,
            revision: response.revision,
            layers: response.layers,
            path_completions,
        });
        if let Some(latest_revision) = self.latest_snapshot_revision {
            self.mark_dirty_editor_stale(latest_revision);
        }
        RemoteFollowUp::None
    }

    fn handle_event_result(
        &mut self,
        generation: u64,
        node: String,
        revision: u64,
    ) -> RemoteFollowUp {
        if generation != self.remote.event_subscription_generation || node != self.node {
            return RemoteFollowUp::None;
        }
        if self.remote.pending_count > 0 {
            self.latest_snapshot_revision = Some(revision);
            if self.is_value_dirty() {
                self.remote.refresh_snapshot_metadata_after_pending = true;
                self.mark_dirty_editor_stale(revision);
            } else {
                self.remote.refresh_snapshot_after_pending = true;
            }
            return RemoteFollowUp::None;
        }
        if self.is_value_dirty() {
            self.mark_dirty_editor_stale(revision);
            RemoteFollowUp::RefreshSnapshotMetadata
        } else {
            self.latest_snapshot_revision = Some(revision);
            RemoteFollowUp::RefreshSnapshot
        }
    }

    fn handle_event_subscription_failed(
        &mut self,
        generation: u64,
        node: String,
    ) -> RemoteFollowUp {
        if generation == self.remote.event_subscription_generation
            && self.remote.event_subscription_node.as_deref() == Some(node.as_str())
        {
            self.remote.event_subscription_node = None;
        }
        RemoteFollowUp::None
    }

    fn handle_value_result(
        &mut self,
        request: ValueRequest,
        result: Result<GetNodeParameterValueResponse, String>,
    ) -> RemoteFollowUp {
        if request.node != self.node || request.path != self.path {
            return RemoteFollowUp::None;
        }

        match result {
            Ok(response) => {
                if let Err(error) = self.apply_value_response(response) {
                    self.clear_value_state();
                    self.status = Status::Error(format!("{error:#}"));
                }
            }
            Err(message) => {
                self.clear_value_state();
                self.status = Status::Error(message);
            }
        }

        RemoteFollowUp::None
    }

    fn handle_set_result(
        &mut self,
        request: SetRequest,
        result: Result<SetNodeParameterResponse, String>,
    ) -> RemoteFollowUp {
        if request.node != self.node
            || request.path != self.path
            || request.target_layer != self.layer
            || Some(request.expected_revision) != self.expected_revision_for_set().ok()
        {
            return RemoteFollowUp::None;
        }

        match result {
            Ok(response) if response.success => {
                self.status =
                    Status::Info("Set parameter value. Refreshing parameter value.".to_string());
                RemoteFollowUp::RefreshAfterSet
            }
            Ok(response) => {
                self.status = Status::Error(format!(
                    "set failed: {}. Refresh and retry if the revision changed.",
                    response.message
                ));
                RemoteFollowUp::None
            }
            Err(message) => {
                self.status = Status::Error(message);
                RemoteFollowUp::None
            }
        }
    }

    fn clear_value_state(&mut self) {
        self.value_revision = None;
        self.latest_snapshot_revision = None;
        self.effective_source_layer.clear();
        self.value_editor.clear();
        self.clean_value_editor.clear();
    }

    fn is_value_dirty(&self) -> bool {
        self.value_editor != self.clean_value_editor
    }

    fn is_stale_against_latest_revision(&self) -> bool {
        matches!(
            (self.value_revision, self.latest_snapshot_revision),
            (Some(base), Some(latest)) if latest > base
        ) || matches!(
            (self.value_revision, self.latest_snapshot_revision),
            (None, Some(_))
        )
    }

    fn mark_dirty_editor_stale(&mut self, latest_revision: u64) {
        self.latest_snapshot_revision = Some(latest_revision);
        let base = self
            .value_revision
            .map(|revision| revision.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        self.status = Status::Error(format!(
            "Parameter changed externally: editing revision {base}, latest revision is {latest_revision}. Refresh to discard local edits."
        ));
    }

    fn render_metadata(&self, ui: &mut Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label("node:");
            ui.monospace(if self.node.is_empty() {
                "none"
            } else {
                &self.node
            });
            ui.separator();
            ui.label("path:");
            ui.monospace(if self.path.is_empty() {
                "none"
            } else {
                &self.path
            });
            ui.separator();
            ui.label("layer:");
            ui.monospace(if self.layer.is_empty() {
                "none"
            } else {
                &self.layer
            });

            if let Some(snapshot) = &self.snapshot {
                ui.separator();
                ui.label("key:");
                ui.monospace(&snapshot.parameter_key);
                ui.separator();
                ui.label("revision:");
                ui.monospace(snapshot.revision.to_string());
            }

            if !self.effective_source_layer.is_empty() {
                ui.separator();
                ui.label("source:");
                ui.monospace(&self.effective_source_layer);
            }
        });
    }

    fn render_status(&self, ui: &mut Ui) {
        match &self.status {
            Status::Idle => {}
            Status::Info(message) => {
                ui.label(message);
            }
            Status::Error(message) => {
                ui.colored_label(ui.visuals().error_fg_color, message);
            }
        }
    }
}

fn parameter_nodes_from_service_names<I, S>(services: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut nodes = services
        .into_iter()
        .filter_map(|service| {
            service
                .as_ref()
                .strip_suffix(PARAMETER_SNAPSHOT_SUFFIX)
                .filter(|node| node.starts_with('/') && !node.is_empty())
                .map(str::to_string)
        })
        .collect::<Vec<_>>();
    nodes.sort();
    nodes.dedup();
    nodes
}

fn parameter_node_completions<I, S>(services: I, active_namespace: &str, input: &str) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let nodes = parameter_nodes_from_service_names(services);
    let absolute = input.starts_with('/');
    let namespace_prefix = completion_namespace_prefix(active_namespace);

    nodes
        .into_iter()
        .filter_map(|node| {
            if absolute {
                Some(node)
            } else {
                node.strip_prefix(&namespace_prefix)
                    .map(ToString::to_string)
            }
        })
        .collect()
}

fn resolve_parameter_node<I, S>(
    services: I,
    active_namespace: &str,
    input: &str,
) -> Result<String, Report>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let input = input.trim();
    if input.is_empty() {
        return Err(eyre!("select a parameter node"));
    }

    let candidate = if input.starts_with('/') {
        input.to_string()
    } else {
        format!("{}{}", completion_namespace_prefix(active_namespace), input)
    };

    let nodes = parameter_nodes_from_service_names(services);
    if nodes.iter().any(|node| node == &candidate) {
        Ok(candidate)
    } else {
        Err(eyre!("parameter node not found: {input}"))
    }
}

fn completion_namespace_prefix(namespace: &str) -> String {
    let namespace = namespace.trim_matches('/');
    if namespace.is_empty() {
        "/".to_string()
    } else {
        format!("/{namespace}/")
    }
}

fn parse_snapshot_path_completions(value_json: &str) -> Result<Vec<String>, Report> {
    let value: Value = serde_json::from_str(value_json)
        .wrap_err("failed to parse parameter snapshot JSON for completion")?;
    Ok(collect_parameter_paths(&value))
}

fn collect_parameter_paths(value: &Value) -> Vec<String> {
    let mut paths = BTreeSet::new();
    collect_parameter_paths_inner("", value, &mut paths);
    paths.into_iter().collect()
}

fn collect_parameter_paths_inner(parent: &str, value: &Value, paths: &mut BTreeSet<String>) {
    let Value::Object(map) = value else {
        return;
    };

    for (key, child) in map {
        let path = if parent.is_empty() {
            key.clone()
        } else {
            format!("{parent}.{key}")
        };
        paths.insert(path.clone());
        collect_parameter_paths_inner(&path, child, paths);
    }
}

fn render_json_for_editor(value_json: &str) -> Result<String, Report> {
    let value: Value =
        serde_json::from_str(value_json).wrap_err("failed to parse parameter JSON")?;
    serde_json::to_string_pretty(&value).wrap_err("failed to render parameter JSON")
}

fn parse_editor_json(input: &str) -> Result<Value, Report> {
    serde_json::from_str(input).wrap_err(
        "invalid JSON value; scalars like 0.72, true, and null are valid, but strings must be quoted",
    )
}

async fn run_remote_operation<T, F>(operation: &'static str, future: F) -> Result<T, String>
where
    F: Future<Output = Result<T, Report>>,
{
    tokio::time::timeout(REMOTE_PARAMETER_TIMEOUT, future)
        .await
        .map_err(|_| {
            format!(
                "timed out while {operation} after {}s",
                REMOTE_PARAMETER_TIMEOUT.as_secs()
            )
        })?
        .map_err(|error| format!("{error:#}"))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        NodeRemoteAction, Panel, ParameterPanel, RemoteFollowUp, RemoteOperationResult,
        RemoteState, SetRequest, SnapshotRequest, SnapshotRequestKind, SnapshotState, Status,
        ValueRequest, collect_parameter_paths, parameter_node_completions,
        parameter_nodes_from_service_names, parse_editor_json, parse_snapshot_path_completions,
        resolve_parameter_node,
    };

    fn snapshot_response(layers: &[&str]) -> ros_z::parameter::GetNodeParametersSnapshotResponse {
        snapshot_response_with_value_json(layers, "{}")
    }

    fn snapshot_response_with_value_json(
        layers: &[&str],
        value_json: &str,
    ) -> ros_z::parameter::GetNodeParametersSnapshotResponse {
        ros_z::parameter::GetNodeParametersSnapshotResponse {
            success: true,
            message: String::new(),
            node_fqn: "/motion/walk".to_string(),
            parameter_key: "walk".to_string(),
            revision: 7,
            committed_at: Default::default(),
            layers: layers.iter().map(|layer| (*layer).to_string()).collect(),
            value_json: value_json.to_string(),
            layer_overlays_json: vec![],
        }
    }

    #[test]
    fn discovers_parameter_nodes_from_snapshot_services() {
        let nodes = parameter_nodes_from_service_names([
            "/motion/walk/parameter/get_snapshot",
            "/vision/ball_detector/parameter/get_snapshot",
            "/vision/ball_detector/parameter/get_snapshot",
            "/motion/walk/parameter/get_value",
        ]);

        assert_eq!(
            nodes,
            vec![
                "/motion/walk".to_string(),
                "/vision/ball_detector".to_string(),
            ]
        );
    }

    #[test]
    fn relative_parameter_node_completion_uses_active_namespace() {
        let completions = parameter_node_completions(
            [
                "/42/motion/walk/parameter/get_snapshot",
                "/42/vision/ball/parameter/get_snapshot",
                "/43/motion/walk/parameter/get_snapshot",
            ],
            "/42",
            "motion",
        );

        assert_eq!(
            completions,
            vec!["motion/walk".to_string(), "vision/ball".to_string()]
        );
    }

    #[test]
    fn absolute_parameter_node_completion_lists_all_namespaces() {
        let completions = parameter_node_completions(
            [
                "/42/motion/walk/parameter/get_snapshot",
                "/43/motion/walk/parameter/get_snapshot",
            ],
            "/42",
            "/",
        );

        assert_eq!(
            completions,
            vec!["/42/motion/walk".to_string(), "/43/motion/walk".to_string()]
        );
    }

    #[test]
    fn relative_parameter_node_selection_resolves_to_absolute_fqn() {
        let resolved = resolve_parameter_node(
            [
                "/42/motion/walk/parameter/get_snapshot",
                "/43/motion/walk/parameter/get_snapshot",
            ],
            "/42",
            "motion/walk",
        )
        .unwrap();

        assert_eq!(resolved, "/42/motion/walk");
    }

    #[test]
    fn invalid_parameter_node_selection_is_rejected_before_remote_call() {
        let error = resolve_parameter_node(
            ["/42/motion/walk/parameter/get_snapshot"],
            "/42",
            "vision/ball",
        )
        .unwrap_err();

        assert!(error.to_string().contains("parameter node not found"));
    }

    #[test]
    fn parse_editor_json_accepts_json_values() {
        assert_eq!(parse_editor_json("0.72").unwrap(), json!(0.72));
        assert_eq!(parse_editor_json("true").unwrap(), json!(true));
        assert_eq!(parse_editor_json("null").unwrap(), json!(null));
        assert_eq!(parse_editor_json(r#"[1, 2]"#).unwrap(), json!([1, 2]));
        assert_eq!(
            parse_editor_json(r#"{"nested":{"count":3}}"#).unwrap(),
            json!({"nested": {"count": 3}})
        );
    }

    #[test]
    fn parse_editor_json_rejects_unquoted_strings_with_context() {
        let error = parse_editor_json("hello").unwrap_err();
        assert!(error.to_string().contains("strings must be quoted"));
    }

    #[test]
    fn parameter_path_completions_include_objects_and_leaves() {
        let paths = collect_parameter_paths(&json!({
            "walk": {
                "speed": 0.7,
                "gait": {
                    "enabled": true
                }
            },
            "enabled": true,
            "array": [1, 2, 3]
        }));

        assert_eq!(
            paths,
            vec![
                "array".to_string(),
                "enabled".to_string(),
                "walk".to_string(),
                "walk.gait".to_string(),
                "walk.gait.enabled".to_string(),
                "walk.speed".to_string(),
            ]
        );
    }

    #[test]
    fn parameter_path_completions_return_empty_for_non_object_root() {
        let paths = collect_parameter_paths(&json!(["not", "a", "parameter", "object"]));

        assert_eq!(paths, Vec::<String>::new());
    }

    #[test]
    fn parameter_path_completions_parse_snapshot_json() {
        let paths = parse_snapshot_path_completions(
            r#"{"walk":{"speed":0.7,"gait":{"enabled":true}},"enabled":true}"#,
        )
        .unwrap();

        assert_eq!(
            paths,
            vec![
                "enabled".to_string(),
                "walk".to_string(),
                "walk.gait".to_string(),
                "walk.gait.enabled".to_string(),
                "walk.speed".to_string(),
            ]
        );

        let error = parse_snapshot_path_completions("not json").unwrap_err();
        assert!(
            error
                .to_string()
                .contains("failed to parse parameter snapshot JSON for completion")
        );
    }

    #[test]
    fn render_json_for_editor_pretty_prints_response_payload() {
        let rendered = super::render_json_for_editor(r#"{"threshold":0.7}"#).unwrap();

        assert_eq!(rendered, "{\n  \"threshold\": 0.7\n}");
    }

    #[test]
    fn snapshot_response_defaults_to_last_active_layer() {
        let mut panel = ParameterPanel::default();

        panel
            .apply_snapshot_response(snapshot_response(&["base", "robot"]))
            .unwrap();

        assert_eq!(panel.layer, "robot");
        assert_eq!(panel.layer_editor, "robot");
        assert_eq!(panel.snapshot.as_ref().unwrap().revision, 7);
    }

    #[test]
    fn snapshot_response_stores_path_completions_from_value_json() {
        let mut panel = ParameterPanel::default();

        panel
            .apply_snapshot_response(snapshot_response_with_value_json(
                &["base", "robot"],
                r#"{"walk":{"speed":0.7,"gait":{"enabled":true}},"enabled":true}"#,
            ))
            .unwrap();

        assert_eq!(
            panel.snapshot.as_ref().unwrap().path_completions,
            vec![
                "enabled".to_string(),
                "walk".to_string(),
                "walk.gait".to_string(),
                "walk.gait.enabled".to_string(),
                "walk.speed".to_string(),
            ]
        );
    }

    #[test]
    fn empty_path_snapshot_renders_full_effective_snapshot() {
        let mut panel = ParameterPanel::default();

        panel
            .apply_snapshot_response(snapshot_response_with_value_json(
                &["base", "robot"],
                r#"{"walk":{"speed":0.7},"enabled":true}"#,
            ))
            .unwrap();

        assert_eq!(
            panel.value_editor,
            "{\n  \"enabled\": true,\n  \"walk\": {\n    \"speed\": 0.7\n  }\n}"
        );
        assert_eq!(panel.clean_value_editor, panel.value_editor);
        assert_eq!(panel.value_revision, Some(7));
        assert_eq!(panel.latest_snapshot_revision, Some(7));
        assert!(panel.effective_source_layer.is_empty());
    }

    #[test]
    fn empty_path_set_remains_rejected() {
        let panel = ParameterPanel {
            node: "/motion/walk".to_string(),
            path: String::new(),
            layer: "robot".to_string(),
            value_revision: Some(7),
            value_editor: "{}".to_string(),
            ..Default::default()
        };

        let error = panel.set_request().unwrap_err();

        assert!(error.to_string().contains("enter a parameter path"));
    }

    #[test]
    fn snapshot_response_rejects_invalid_value_json_without_replacing_snapshot() {
        let mut panel = ParameterPanel {
            snapshot: Some(SnapshotState {
                parameter_key: "old".to_string(),
                revision: 3,
                layers: vec!["old-layer".to_string()],
                path_completions: vec!["old.path".to_string()],
            }),
            ..Default::default()
        };

        let error = panel
            .apply_snapshot_response(snapshot_response_with_value_json(&["base"], "{"))
            .unwrap_err();

        assert!(error.to_string().contains("parameter snapshot JSON"));
        assert_eq!(panel.snapshot.as_ref().unwrap().parameter_key, "old");
        assert_eq!(
            panel.snapshot.as_ref().unwrap().path_completions,
            vec!["old.path".to_string()]
        );
    }

    #[test]
    fn commit_node_reports_change_and_clears_snapshot_value_state() {
        let mut panel = ParameterPanel {
            node_editor: "/motion/walk".to_string(),
            node: "/vision/ball".to_string(),
            path_editor: "walk.speed".to_string(),
            path: "walk.speed".to_string(),
            snapshot: Some(SnapshotState {
                parameter_key: "ball".to_string(),
                revision: 4,
                layers: vec!["robot".to_string()],
                path_completions: vec!["threshold".to_string()],
            }),
            value_revision: Some(4),
            effective_source_layer: "robot".to_string(),
            value_editor: "0.4".to_string(),
            ..Default::default()
        };

        assert!(panel.commit_node(&["/motion/walk/parameter/get_snapshot".to_string()], "/"));
        assert_eq!(panel.node, "/motion/walk");
        assert!(panel.path.is_empty());
        assert!(panel.path_editor.is_empty());
        assert!(panel.snapshot.is_none());
        assert_eq!(panel.value_revision, None);
        assert!(panel.effective_source_layer.is_empty());
        assert!(panel.value_editor.is_empty());
    }

    #[test]
    fn remote_action_node_validation_rejects_invalid_editor_without_using_committed_node() {
        let mut panel = ParameterPanel {
            node_editor: "/vision/ball".to_string(),
            node: "/motion/walk".to_string(),
            ..Default::default()
        };

        let result = panel.validate_node_for_remote_action(
            &["/motion/walk/parameter/get_snapshot".to_string()],
            "/",
        );

        assert_eq!(result, NodeRemoteAction::Invalid);
        assert_eq!(panel.node, "/motion/walk");
        assert!(matches!(panel.status, Status::Error(_)));
    }

    #[test]
    fn remote_action_node_validation_commits_changed_editor_without_using_old_node() {
        let mut panel = ParameterPanel {
            node_editor: "/vision/ball".to_string(),
            node: "/motion/walk".to_string(),
            path_editor: "walk.speed".to_string(),
            path: "walk.speed".to_string(),
            snapshot: Some(SnapshotState {
                parameter_key: "walk".to_string(),
                revision: 7,
                layers: vec!["robot".to_string()],
                path_completions: vec!["walk.speed".to_string()],
            }),
            value_revision: Some(7),
            value_editor: "0.7".to_string(),
            ..Default::default()
        };

        let result = panel.validate_node_for_remote_action(
            &[
                "/motion/walk/parameter/get_snapshot".to_string(),
                "/vision/ball/parameter/get_snapshot".to_string(),
            ],
            "/",
        );

        assert_eq!(result, NodeRemoteAction::Changed);
        assert_eq!(panel.node, "/vision/ball");
        assert!(panel.path.is_empty());
        assert!(panel.path_editor.is_empty());
        assert!(panel.snapshot.is_none());
        assert_eq!(panel.value_revision, None);
        assert!(panel.value_editor.is_empty());
    }

    #[test]
    fn remote_action_node_validation_accepts_current_editor() {
        let mut panel = ParameterPanel {
            node_editor: "motion/walk".to_string(),
            node: "/42/motion/walk".to_string(),
            ..Default::default()
        };

        let result = panel.validate_node_for_remote_action(
            &["/42/motion/walk/parameter/get_snapshot".to_string()],
            "/42",
        );

        assert_eq!(result, NodeRemoteAction::Current);
        assert_eq!(panel.node, "/42/motion/walk");
    }

    #[test]
    fn commit_path_reports_no_change_for_same_path() {
        let mut panel = ParameterPanel {
            path_editor: "walk.speed".to_string(),
            path: "walk.speed".to_string(),
            value_revision: Some(4),
            value_editor: "0.7".to_string(),
            ..Default::default()
        };

        assert!(!panel.commit_path());
        assert_eq!(panel.value_revision, Some(4));
        assert_eq!(panel.value_editor, "0.7");
    }

    #[test]
    fn stale_snapshot_result_does_not_replace_current_snapshot() {
        let mut panel = ParameterPanel {
            node: "/vision/ball".to_string(),
            snapshot: Some(SnapshotState {
                parameter_key: "old".to_string(),
                revision: 3,
                layers: vec!["old-layer".to_string()],
                path_completions: vec!["old.path".to_string()],
            }),
            ..Default::default()
        };

        let follow_up = panel.handle_remote_result(RemoteOperationResult::Snapshot {
            request: SnapshotRequest {
                node: "/motion/walk".to_string(),
                kind: SnapshotRequestKind::Auto,
            },
            result: Ok(snapshot_response_with_value_json(
                &["base"],
                r#"{"new":{"path":1}}"#,
            )),
        });

        assert_eq!(follow_up, RemoteFollowUp::None);
        assert_eq!(panel.snapshot.as_ref().unwrap().parameter_key, "old");
        assert_eq!(
            panel.snapshot.as_ref().unwrap().path_completions,
            vec!["old.path".to_string()]
        );
    }

    #[test]
    fn stale_value_result_does_not_replace_current_editor() {
        let mut panel = ParameterPanel {
            node: "/motion/walk".to_string(),
            path: "walk.speed".to_string(),
            value_editor: "0.7".to_string(),
            value_revision: Some(7),
            ..Default::default()
        };

        let follow_up = panel.handle_remote_result(RemoteOperationResult::Value {
            request: ValueRequest {
                node: "/motion/walk".to_string(),
                path: "walk.gait.enabled".to_string(),
            },
            result: Ok(ros_z::parameter::GetNodeParameterValueResponse {
                success: true,
                message: String::new(),
                revision: 8,
                path: "walk.gait.enabled".to_string(),
                effective_source_layer: "robot".to_string(),
                value_json: "true".to_string(),
            }),
        });

        assert_eq!(follow_up, RemoteFollowUp::None);
        assert_eq!(panel.value_editor, "0.7");
        assert_eq!(panel.value_revision, Some(7));
    }

    #[test]
    fn clean_editor_event_schedules_snapshot_refresh() {
        let mut panel = ParameterPanel {
            node: "/motion/walk".to_string(),
            remote: RemoteState {
                event_subscription_generation: 1,
                ..Default::default()
            },
            value_editor: "0.7".to_string(),
            clean_value_editor: "0.7".to_string(),
            value_revision: Some(7),
            ..Default::default()
        };

        let follow_up = panel.handle_remote_result(RemoteOperationResult::Event {
            generation: 1,
            node: "/motion/walk".to_string(),
            revision: 8,
        });

        assert_eq!(follow_up, RemoteFollowUp::RefreshSnapshot);
    }

    #[test]
    fn old_node_event_is_ignored() {
        let mut panel = ParameterPanel {
            node: "/motion/walk".to_string(),
            remote: RemoteState {
                event_subscription_generation: 1,
                ..Default::default()
            },
            value_editor: "0.7".to_string(),
            clean_value_editor: "0.7".to_string(),
            value_revision: Some(7),
            ..Default::default()
        };

        let follow_up = panel.handle_remote_result(RemoteOperationResult::Event {
            generation: 1,
            node: "/vision/ball".to_string(),
            revision: 8,
        });

        assert_eq!(follow_up, RemoteFollowUp::None);
        assert_eq!(panel.latest_snapshot_revision, None);
    }

    #[test]
    fn dirty_editor_event_marks_stale_without_overwriting_text() {
        let mut panel = ParameterPanel {
            node: "/motion/walk".to_string(),
            remote: RemoteState {
                event_subscription_generation: 1,
                ..Default::default()
            },
            value_editor: "0.8".to_string(),
            clean_value_editor: "0.7".to_string(),
            value_revision: Some(7),
            ..Default::default()
        };

        let follow_up = panel.handle_remote_result(RemoteOperationResult::Event {
            generation: 1,
            node: "/motion/walk".to_string(),
            revision: 9,
        });

        assert_eq!(follow_up, RemoteFollowUp::RefreshSnapshotMetadata);
        assert_eq!(panel.value_editor, "0.8");
        assert_eq!(panel.value_revision, Some(7));
        assert_eq!(panel.latest_snapshot_revision, Some(9));
        assert!(matches!(panel.status, Status::Error(_)));
    }

    #[test]
    fn dirty_editor_without_base_revision_is_stale_after_known_latest_revision() {
        let panel = ParameterPanel {
            node: "/motion/walk".to_string(),
            path: "walk.speed".to_string(),
            layer: "robot".to_string(),
            snapshot: Some(SnapshotState {
                parameter_key: "walk".to_string(),
                revision: 7,
                layers: vec!["robot".to_string()],
                path_completions: Vec::new(),
            }),
            value_editor: "0.8".to_string(),
            clean_value_editor: "0.7".to_string(),
            value_revision: None,
            latest_snapshot_revision: Some(9),
            ..Default::default()
        };

        let error = panel.set_request().unwrap_err();

        assert!(error.to_string().contains("changed externally"));
    }

    #[test]
    fn current_event_subscription_failure_clears_latched_node_for_retry() {
        let mut panel = ParameterPanel {
            node: "/motion/walk".to_string(),
            remote: RemoteState {
                event_subscription_node: Some("/motion/walk".to_string()),
                event_subscription_generation: 3,
                ..Default::default()
            },
            ..Default::default()
        };

        let follow_up =
            panel.handle_remote_result(RemoteOperationResult::EventSubscriptionFailed {
                generation: 3,
                node: "/motion/walk".to_string(),
            });

        assert_eq!(follow_up, RemoteFollowUp::None);
        assert_eq!(panel.remote.event_subscription_node, None);
        assert_eq!(panel.remote.event_subscription_generation, 3);
    }

    #[test]
    fn stale_event_subscription_failure_keeps_current_latched_node() {
        let mut panel = ParameterPanel {
            node: "/motion/walk".to_string(),
            remote: RemoteState {
                event_subscription_node: Some("/motion/walk".to_string()),
                event_subscription_generation: 4,
                ..Default::default()
            },
            ..Default::default()
        };

        let follow_up =
            panel.handle_remote_result(RemoteOperationResult::EventSubscriptionFailed {
                generation: 3,
                node: "/motion/walk".to_string(),
            });

        assert_eq!(follow_up, RemoteFollowUp::None);
        assert_eq!(
            panel.remote.event_subscription_node,
            Some("/motion/walk".to_string())
        );
        assert_eq!(panel.remote.event_subscription_generation, 4);
    }

    #[test]
    fn manual_snapshot_then_value_refresh_can_replace_dirty_editor() {
        let mut panel = ParameterPanel {
            node: "/motion/walk".to_string(),
            path: "walk.speed".to_string(),
            value_editor: "0.8".to_string(),
            clean_value_editor: "0.7".to_string(),
            value_revision: Some(7),
            latest_snapshot_revision: Some(9),
            ..Default::default()
        };

        panel
            .apply_value_response(ros_z::parameter::GetNodeParameterValueResponse {
                success: true,
                message: String::new(),
                revision: 9,
                path: "walk.speed".to_string(),
                effective_source_layer: "robot".to_string(),
                value_json: "0.9".to_string(),
            })
            .unwrap();

        assert_eq!(panel.value_editor, "0.9");
        assert_eq!(panel.clean_value_editor, "0.9");
        assert_eq!(panel.value_revision, Some(9));
    }

    #[test]
    fn event_during_pending_operation_defers_refresh_until_pending_drains() {
        let mut panel = ParameterPanel {
            node: "/motion/walk".to_string(),
            remote: RemoteState {
                pending_count: 1,
                event_subscription_generation: 1,
                ..Default::default()
            },
            value_editor: "0.7".to_string(),
            clean_value_editor: "0.7".to_string(),
            value_revision: Some(7),
            ..Default::default()
        };

        let follow_up = panel.handle_remote_result(RemoteOperationResult::Event {
            generation: 1,
            node: "/motion/walk".to_string(),
            revision: 8,
        });

        assert_eq!(follow_up, RemoteFollowUp::None);
        assert!(panel.remote.refresh_snapshot_after_pending);
    }

    #[test]
    fn set_error_keeps_editor_contents() {
        let mut panel = ParameterPanel {
            node: "/motion/walk".to_string(),
            path: "walk.speed".to_string(),
            layer: "robot".to_string(),
            value_editor: "0.7".to_string(),
            value_revision: Some(7),
            ..Default::default()
        };

        let follow_up = panel.handle_remote_result(RemoteOperationResult::Set {
            request: SetRequest {
                node: "/motion/walk".to_string(),
                path: "walk.speed".to_string(),
                target_layer: "robot".to_string(),
                expected_revision: 7,
            },
            result: Ok(ros_z::parameter::SetNodeParameterResponse {
                success: false,
                message: "revision mismatch".to_string(),
                committed_revision: 8,
                changed_paths: Vec::new(),
            }),
        });

        assert_eq!(follow_up, RemoteFollowUp::None);
        assert_eq!(panel.value_editor, "0.7");
        assert!(matches!(panel.status, Status::Error(_)));
    }

    #[test]
    fn value_editor_can_only_be_edited_without_pending_remote_operations() {
        let mut panel = ParameterPanel {
            path: "walk.speed".to_string(),
            ..Default::default()
        };

        assert!(panel.can_edit_value());

        panel.remote.pending_count = 1;

        assert!(!panel.can_edit_value());
    }

    #[test]
    fn empty_path_value_editor_is_read_only() {
        let panel = ParameterPanel::default();

        assert!(!panel.can_edit_value());
    }

    #[test]
    fn selectors_can_only_be_edited_without_pending_remote_operations() {
        let mut panel = ParameterPanel::default();

        assert!(panel.can_edit_selectors());

        panel.remote.pending_count = 1;

        assert!(!panel.can_edit_selectors());
    }

    #[test]
    fn remote_operations_can_only_start_without_pending_remote_operations() {
        let mut panel = ParameterPanel::default();

        assert!(panel.can_start_remote_operation());

        panel.remote.pending_count = 1;

        assert!(!panel.can_start_remote_operation());
    }

    #[test]
    fn snapshot_response_reports_missing_selected_layer_without_retargeting() {
        let mut panel = ParameterPanel {
            layer: "retired".to_string(),
            layer_editor: "retired".to_string(),
            value_revision: Some(6),
            effective_source_layer: "retired".to_string(),
            value_editor: "0.9".to_string(),
            ..Default::default()
        };

        let error = panel
            .apply_snapshot_response(snapshot_response(&["base", "robot"]))
            .unwrap_err();

        assert!(error.to_string().contains("retired"));
        assert!(error.to_string().contains("not active"));
        assert_eq!(panel.layer, "retired");
        assert_eq!(panel.layer_editor, "retired");
        assert_eq!(
            panel.snapshot.as_ref().unwrap().layers,
            vec!["base".to_string(), "robot".to_string()]
        );
        assert_eq!(panel.value_revision, None);
        assert!(panel.effective_source_layer.is_empty());
        assert!(panel.value_editor.is_empty());
    }

    #[test]
    fn set_requires_fetched_revision() {
        let panel = ParameterPanel {
            path: "threshold".to_string(),
            layer: "robot".to_string(),
            value_editor: "0.9".to_string(),
            ..Default::default()
        };

        let error = panel.expected_revision_for_set().unwrap_err();

        assert!(error.to_string().contains("refresh"));
    }

    #[test]
    fn set_uses_fetched_value_revision_before_snapshot_revision() {
        let panel = ParameterPanel {
            value_revision: Some(9),
            snapshot: Some(SnapshotState {
                parameter_key: "walk".to_string(),
                revision: 7,
                layers: vec!["robot".to_string()],
                path_completions: Vec::new(),
            }),
            ..Default::default()
        };

        assert_eq!(panel.expected_revision_for_set().unwrap(), 9);
    }

    #[test]
    fn set_can_use_fetched_snapshot_revision() {
        let panel = ParameterPanel {
            snapshot: Some(SnapshotState {
                parameter_key: "walk".to_string(),
                revision: 7,
                layers: vec!["robot".to_string()],
                path_completions: Vec::new(),
            }),
            ..Default::default()
        };

        assert_eq!(panel.expected_revision_for_set().unwrap(), 7);
    }

    #[test]
    fn value_response_updates_editor_revision_and_source_layer() {
        let mut panel = ParameterPanel::default();

        panel
            .apply_value_response(ros_z::parameter::GetNodeParameterValueResponse {
                success: true,
                message: String::new(),
                revision: 9,
                path: "threshold".to_string(),
                effective_source_layer: "robot".to_string(),
                value_json: "0.9".to_string(),
            })
            .unwrap();

        assert_eq!(panel.value_editor, "0.9");
        assert_eq!(panel.clean_value_editor, panel.value_editor);
        assert_eq!(panel.value_revision, Some(9));
        assert_eq!(panel.effective_source_layer, "robot");
    }

    #[test]
    fn save_preserves_node_path_and_layer() {
        let panel = ParameterPanel {
            node: "/motion/walk".to_string(),
            path: "threshold".to_string(),
            layer: "parameters/ros_z/robot/42".to_string(),
            ..Default::default()
        };

        assert_eq!(
            panel.save(),
            json!({
                "node": "/motion/walk",
                "path": "threshold",
                "layer": "parameters/ros_z/robot/42",
            })
        );
    }

    #[test]
    fn saved_state_round_trips_through_json() {
        let saved = super::SavedState {
            node: "/motion/walk".to_string(),
            path: "nested.count".to_string(),
            layer: "parameters/ros_z/robot/42".to_string(),
        };

        let value = serde_json::to_value(&saved).unwrap();
        let restored: super::SavedState = serde_json::from_value(value).unwrap();

        assert_eq!(restored, saved);
    }
}
