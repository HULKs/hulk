use std::{future::Future, time::Duration};

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
    },
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::panel::{Panel, PanelCreationContext, PanelUiContext};

const PARAMETER_SNAPSHOT_SUFFIX: &str = "/parameter/get_snapshot";
const REMOTE_PARAMETER_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Default)]
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SnapshotState {
    parameter_key: String,
    revision: u64,
    layers: Vec<String>,
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
            snapshot: None,
            value_revision: None,
            effective_source_layer: String::new(),
            value_editor: String::new(),
            status: Status::Idle,
        }
    }

    fn ui(&mut self, ui: &mut Ui, context: PanelUiContext<'_>) {
        let nodes = parameter_nodes_from_graph(context.backend.graph());
        let layers = self
            .snapshot
            .as_ref()
            .map(|snapshot| snapshot.layers.clone())
            .unwrap_or_default();

        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label("Node");
                let node_response = ui.add(CompletionEdit::new(
                    ui.id().with("parameter_node"),
                    &nodes,
                    &mut self.node_editor,
                ));
                if node_response.changed() || node_response.lost_focus() {
                    self.commit_node();
                }

                ui.label("Path");
                let path_response = ui.text_edit_singleline(&mut self.path_editor);
                if path_response.lost_focus() {
                    self.commit_path();
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

                if ui.button("Refresh").clicked() {
                    self.refresh(&context);
                }

                let can_set = !self.node_editor.trim().is_empty()
                    && !self.path_editor.trim().is_empty()
                    && !self.layer_editor.trim().is_empty()
                    && !self.value_editor.trim().is_empty();
                ui.add_enabled_ui(can_set, |ui| {
                    if ui.button("Set").clicked() {
                        self.set_value(&context);
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
                ui.add(
                    TextEdit::multiline(&mut self.value_editor)
                        .font(eframe::egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .code_editor(),
                );
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
    fn commit_node(&mut self) {
        let next = self.node_editor.trim().to_string();
        if next == self.node {
            return;
        }
        self.node = next;
        self.snapshot = None;
        self.value_revision = None;
        self.effective_source_layer.clear();
        self.value_editor.clear();
        self.status = Status::Idle;
    }

    fn commit_path(&mut self) {
        let next = self.path_editor.trim().to_string();
        if next == self.path {
            return;
        }
        self.path = next;
        self.value_revision = None;
        self.effective_source_layer.clear();
        self.value_editor.clear();
        self.status = Status::Idle;
    }

    fn commit_layer(&mut self) {
        self.layer = self.layer_editor.trim().to_string();
    }

    fn client(&self, context: &PanelUiContext<'_>) -> Result<RemoteParameterClient, Report> {
        if self.node.is_empty() {
            return Err(eyre!("select a parameter node"));
        }
        RemoteParameterClient::new(context.backend.node(), self.node.clone())
            .wrap_err("failed to create remote parameter client")
    }

    fn refresh(&mut self, context: &PanelUiContext<'_>) {
        self.commit_node();
        self.commit_path();
        self.commit_layer();

        if let Err(error) = self
            .fetch_snapshot(context)
            .and_then(|_| self.fetch_value(context))
        {
            self.status = Status::Error(format!("{error:#}"));
        }
    }

    fn fetch_snapshot(&mut self, context: &PanelUiContext<'_>) -> Result<(), Report> {
        let client = self.client(context)?;
        let response = block_on_remote(context, "fetching parameter snapshot", async {
            client
                .get_snapshot()
                .await
                .wrap_err("failed to fetch parameter snapshot")
        })?;
        self.apply_snapshot_response(response)
    }

    fn apply_snapshot_response(
        &mut self,
        response: GetNodeParametersSnapshotResponse,
    ) -> Result<(), Report> {
        if !response.success {
            return Err(eyre!("{}", response.message));
        }

        let default_layer = response.layers.last().cloned();
        let selected_layer = self.layer.clone();
        let selected_layer_missing =
            !selected_layer.is_empty() && !response.layers.contains(&selected_layer);

        self.snapshot = Some(SnapshotState {
            parameter_key: response.parameter_key,
            revision: response.revision,
            layers: response.layers,
        });

        if selected_layer_missing {
            self.value_revision = None;
            self.effective_source_layer.clear();
            self.value_editor.clear();
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

    fn fetch_value(&mut self, context: &PanelUiContext<'_>) -> Result<(), Report> {
        if self.path.is_empty() {
            return Ok(());
        }

        let client = self.client(context)?;
        let response = block_on_remote(context, "fetching parameter value", async {
            client
                .get_value(self.path.clone())
                .await
                .wrap_err("failed to fetch parameter value")
        })?;
        self.apply_value_response(response)
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

    fn set_value(&mut self, context: &PanelUiContext<'_>) {
        self.commit_node();
        self.commit_path();
        self.commit_layer();

        let result = self.try_set_value(context);
        match result {
            Ok(()) => self.status = Status::Info("Set parameter value.".to_string()),
            Err(error) => self.status = Status::Error(format!("{error:#}")),
        }
    }

    fn try_set_value(&mut self, context: &PanelUiContext<'_>) -> Result<(), Report> {
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

        let value = parse_editor_json(&self.value_editor)?;
        let expected_revision = self.expected_revision_for_set()?;
        let client = self.client(context)?;
        let response = block_on_remote(context, "setting parameter value", async {
            client
                .set_json(
                    self.path.clone(),
                    &value,
                    self.layer.clone(),
                    Some(expected_revision),
                )
                .await
                .wrap_err("failed to set parameter value")
        })?;

        if !response.success {
            return Err(eyre!(
                "set failed: {}. Refresh and retry if the revision changed.",
                response.message
            ));
        }

        self.fetch_snapshot(context)?;
        self.fetch_value(context)?;
        Ok(())
    }

    fn expected_revision_for_set(&self) -> Result<u64, Report> {
        self.value_revision
            .or_else(|| self.snapshot.as_ref().map(|snapshot| snapshot.revision))
            .ok_or_else(|| eyre!("refresh the parameter snapshot or value before setting"))
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
    let services = graph
        .view()
        .service_names_and_types()
        .into_iter()
        .map(|(name, _type_name)| name);
    parameter_nodes_from_service_names(services)
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

fn block_on_remote<T, F>(
    context: &PanelUiContext<'_>,
    operation: &'static str,
    future: F,
) -> Result<T, Report>
where
    F: Future<Output = Result<T, Report>>,
{
    context
        .backend
        .runtime_handle()
        .block_on(async { tokio::time::timeout(REMOTE_PARAMETER_TIMEOUT, future).await })
        .map_err(|_| {
            eyre!(
                "timed out while {operation} after {}s",
                REMOTE_PARAMETER_TIMEOUT.as_secs()
            )
        })?
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        Panel, ParameterPanel, SnapshotState, parameter_nodes_from_service_names, parse_editor_json,
    };

    fn snapshot_response(layers: &[&str]) -> ros_z::parameter::GetNodeParametersSnapshotResponse {
        ros_z::parameter::GetNodeParametersSnapshotResponse {
            success: true,
            message: String::new(),
            node_fqn: "/motion/walk".to_string(),
            parameter_key: "walk".to_string(),
            revision: 7,
            committed_at: Default::default(),
            layers: layers.iter().map(|layer| (*layer).to_string()).collect(),
            value_json: "{}".to_string(),
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
