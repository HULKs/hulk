use std::{collections::BTreeSet, future::Future, time::Duration};

use color_eyre::{
    Report,
    eyre::{Context as _, eyre},
};
use eframe::egui::{ScrollArea, TextEdit, Ui};
use hulk_widgets::CompletionEdit;
use ros_z::{
    graph::Graph,
    parameter::{
        GetNodeParameterValueResponse, GetNodeParametersSnapshotResponse, RemoteParameterClient,
        SetNodeParameterResponse,
    },
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
    Value {
        request: ValueRequest,
        result: Result<GetNodeParameterValueResponse, String>,
    },
    Set {
        request: SetRequest,
        result: Result<SetNodeParameterResponse, String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RemoteFollowUp {
    None,
    FetchValue,
    RefreshAfterSet,
}

struct RemoteState {
    sender: UnboundedSender<RemoteOperationResult>,
    receiver: UnboundedReceiver<RemoteOperationResult>,
    pending_count: usize,
}

impl Default for RemoteState {
    fn default() -> Self {
        let (sender, receiver) = unbounded_channel();
        Self {
            sender,
            receiver,
            pending_count: 0,
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

        let nodes = parameter_nodes_from_graph(context.backend.graph());
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
                    if (node_response.changed() || node_response.lost_focus()) && self.commit_node()
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
                        self.commit_node();
                        self.commit_path();
                        self.commit_layer();
                        self.spawn_snapshot_fetch(&context, SnapshotRequestKind::Manual);
                    }
                });

                let can_set = self.can_start_remote_operation()
                    && !self.node_editor.trim().is_empty()
                    && !self.path_editor.trim().is_empty()
                    && !self.layer_editor.trim().is_empty()
                    && !self.value_editor.trim().is_empty();
                ui.add_enabled_ui(can_set, |ui| {
                    if ui.button("Set").clicked() {
                        self.spawn_set_value(&context);
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
    fn commit_node(&mut self) -> bool {
        let next = self.node_editor.trim().to_string();
        if next == self.node {
            return false;
        }
        self.node = next;
        self.snapshot = None;
        self.clear_value_state();
        self.status = Status::Idle;
        true
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
        self.can_start_remote_operation()
    }

    fn can_start_remote_operation(&self) -> bool {
        self.remote.pending_count == 0
    }

    fn drain_remote_results(&mut self, context: &PanelUiContext<'_>) {
        while let Ok(result) = self.remote.receiver.try_recv() {
            self.remote.pending_count = self.remote.pending_count.saturating_sub(1);
            match self.handle_remote_result(result) {
                RemoteFollowUp::None => {}
                RemoteFollowUp::FetchValue => self.spawn_value_fetch(context),
                RemoteFollowUp::RefreshAfterSet => {
                    self.spawn_snapshot_fetch(context, SnapshotRequestKind::AfterSet)
                }
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
        let default_layer = response.layers.last().cloned();
        let selected_layer = self.layer.clone();
        let selected_layer_missing =
            !selected_layer.is_empty() && !response.layers.contains(&selected_layer);

        self.snapshot = Some(SnapshotState {
            parameter_key: response.parameter_key,
            revision: response.revision,
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
        self.value_revision = Some(response.revision);
        self.effective_source_layer = response.effective_source_layer;
        self.status = Status::Info("Fetched parameter value.".to_string());
        Ok(())
    }

    fn spawn_set_value(&mut self, context: &PanelUiContext<'_>) {
        self.commit_node();
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
            RemoteOperationResult::Value { request, result } => {
                self.handle_value_result(request, result)
            }
            RemoteOperationResult::Set { request, result } => {
                self.handle_set_result(request, result)
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
        self.effective_source_layer.clear();
        self.value_editor.clear();
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

fn parameter_nodes_from_graph(graph: &Graph) -> Vec<String> {
    let graph = graph.lock();
    let services = graph.services().map(|service| service.topic.as_str());
    parameter_nodes_from_service_names(services)
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
        Panel, ParameterPanel, RemoteFollowUp, RemoteOperationResult, SetRequest, SnapshotRequest,
        SnapshotRequestKind, SnapshotState, Status, ValueRequest, collect_parameter_paths,
        parameter_nodes_from_service_names, parse_editor_json, parse_snapshot_path_completions,
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

        assert!(panel.commit_node());
        assert_eq!(panel.node, "/motion/walk");
        assert!(panel.snapshot.is_none());
        assert_eq!(panel.value_revision, None);
        assert!(panel.effective_source_layer.is_empty());
        assert!(panel.value_editor.is_empty());
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
        let mut panel = ParameterPanel::default();

        assert!(panel.can_edit_value());

        panel.remote.pending_count = 1;

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
