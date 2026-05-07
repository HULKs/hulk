use std::sync::Arc;

use eframe::egui::{ComboBox, Key, Response, ScrollArea, TextEdit, Ui, Widget};
use hulk_widgets::CompletionEdit;
use serde_json::{Value, json};

use crate::{
    panel::{Panel, PanelCreationContext},
    robot::Robot,
};

pub struct ParameterPanel {
    robot: Arc<Robot>,
    node_selector: String,
    field_path: String,
    target_layer: String,
    available_layers: Vec<String>,
    config_key: Option<String>,
    field_paths: Vec<String>,
    field_metadata: Option<ParameterFieldMetadata>,
    parameter_value: String,
    current_revision: Option<u64>,
    effective_source_layer: Option<String>,
    manual_field_path_dirty: bool,
    status_message: Option<String>,
}

struct ParameterFieldMetadata {
    type_name: String,
    writable: bool,
    effective_source_layer: String,
}

impl<'a> Panel<'a> for ParameterPanel {
    const NAME: &'static str = "Parameter";

    fn new(context: PanelCreationContext) -> Self {
        let mut panel = Self {
            robot: context.robot,
            node_selector: context
                .value
                .and_then(|value| value.get("node"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            field_path: context
                .value
                .and_then(|value| value.get("field_path"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            target_layer: context
                .value
                .and_then(|value| value.get("layer"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            available_layers: Vec::new(),
            config_key: None,
            field_paths: Vec::new(),
            field_metadata: None,
            parameter_value: String::new(),
            current_revision: None,
            effective_source_layer: None,
            manual_field_path_dirty: false,
            status_message: None,
        };
        panel.refresh_node_context();
        panel.refresh_selected_value();
        panel
    }

    fn save(&self) -> Value {
        json!({
            "node": self.node_selector,
            "field_path": self.field_path,
            "layer": self.target_layer,
        })
    }
}

impl ParameterPanel {
    fn refresh_node_context(&mut self) {
        self.field_paths.clear();
        self.field_metadata = None;
        self.available_layers.clear();
        self.config_key = None;
        self.status_message = None;

        if self.node_selector.is_empty() {
            return;
        }

        match self.robot.get_config_snapshot(&self.node_selector) {
            Ok(response) if response.success => {
                self.available_layers = response.layers;
                self.config_key = Some(response.parameter_key);
                self.current_revision = Some(response.revision);
                match serde_json::from_str::<Value>(&response.value_json) {
                    Ok(value) => {
                        self.field_paths = flatten_parameter_paths(&value);
                    }
                    Err(error) => {
                        self.status_message = Some(format!("invalid snapshot JSON: {error}"));
                    }
                }
                if self.target_layer.is_empty()
                    || !self
                        .available_layers
                        .iter()
                        .any(|layer| layer == &self.target_layer)
                {
                    self.target_layer = self.available_layers.last().cloned().unwrap_or_default();
                }
            }
            Ok(response) => {
                self.status_message = Some(response.message);
            }
            Err(error) => {
                self.status_message = Some(error.to_string());
            }
        }
    }

    fn refresh_selected_value(&mut self) {
        self.current_revision = None;
        self.effective_source_layer = None;
        self.field_metadata = None;

        if self.node_selector.is_empty() || self.field_path.is_empty() {
            return;
        }

        match self
            .robot
            .get_config_value(&self.node_selector, &self.field_path)
        {
            Ok(response) if response.success => {
                let source_layer = response.effective_source_layer;
                self.current_revision = Some(response.revision);
                self.field_metadata = Some(ParameterFieldMetadata {
                    type_name: "json".to_string(),
                    writable: true,
                    effective_source_layer: source_layer.clone(),
                });
                self.effective_source_layer = Some(source_layer);
                self.parameter_value = response.value_json;
                self.status_message = None;
            }
            Ok(response) => {
                self.status_message = Some(response.message);
            }
            Err(error) => {
                self.status_message = Some(error.to_string());
            }
        }
    }

    fn commit(&mut self) {
        let value = match serde_json::from_str::<Value>(&self.parameter_value) {
            Ok(value) => value,
            Err(error) => {
                self.status_message = Some(format!("invalid JSON value: {error}"));
                return;
            }
        };

        let expected_revision = self.current_revision;
        match self.robot.set_config_json(
            &self.node_selector,
            &self.field_path,
            &value,
            self.target_layer.clone(),
            expected_revision,
        ) {
            Ok(response) if response.success => {
                let message = format!("Committed revision {}", response.committed_revision);
                self.refresh_selected_value();
                self.status_message = Some(message);
            }
            Ok(response) => {
                self.status_message = Some(response.message);
            }
            Err(error) => {
                self.status_message = Some(error.to_string());
            }
        }
    }

    fn reset_scope(&mut self) {
        match self.robot.reset_config(
            &self.node_selector,
            &self.field_path,
            self.target_layer.clone(),
            self.current_revision,
        ) {
            Ok(response) if response.success => {
                let message = format!("Reset committed revision {}", response.committed_revision);
                self.refresh_selected_value();
                self.status_message = Some(message);
            }
            Ok(response) => {
                self.status_message = Some(response.message);
            }
            Err(error) => {
                self.status_message = Some(error.to_string());
            }
        }
    }
}

fn flatten_parameter_paths(value: &Value) -> Vec<String> {
    let mut paths = Vec::new();
    flatten_parameter_paths_into(value, String::new(), &mut paths);
    paths.sort();
    paths
}

fn flatten_parameter_paths_into(value: &Value, prefix: String, paths: &mut Vec<String>) {
    let Value::Object(fields) = value else {
        return;
    };

    for (key, value) in fields {
        let path = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        paths.push(path.clone());
        flatten_parameter_paths_into(value, path, paths);
    }
}

fn should_fetch_manual_field_path(
    dirty: bool,
    lost_focus: bool,
    has_focus: bool,
    enter_pressed: bool,
) -> bool {
    dirty && (lost_focus || (has_focus && enter_pressed))
}

fn format_field_metadata(metadata: &ParameterFieldMetadata) -> String {
    format!(
        "type={} writable={} source={}",
        metadata.type_name, metadata.writable, metadata.effective_source_layer
    )
}

impl Widget for &mut ParameterPanel {
    fn ui(self, ui: &mut Ui) -> Response {
        let node_state = self.robot.config_node_list_state();
        let node_names = node_state
            .nodes
            .iter()
            .map(|node| node.node_fqn.clone())
            .collect::<Vec<_>>();

        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                let node_response = ui.add(CompletionEdit::new(
                    ui.id().with("parameter-node"),
                    &node_names,
                    &mut self.node_selector,
                ));
                if node_state.discovering {
                    ui.label("discovering parameter nodes...");
                }
                if node_response.changed() {
                    self.refresh_node_context();
                    self.refresh_selected_value();
                }

                if !self.field_paths.is_empty() {
                    let field_response = ui.add(CompletionEdit::new(
                        ui.id().with("parameter-field"),
                        &self.field_paths,
                        &mut self.field_path,
                    ));
                    if field_response.changed() {
                        self.manual_field_path_dirty = false;
                        self.refresh_selected_value();
                    }
                } else {
                    let field_response = ui.add(TextEdit::singleline(&mut self.field_path));
                    if field_response.changed() {
                        self.manual_field_path_dirty = true;
                    }
                    if should_fetch_manual_field_path(
                        self.manual_field_path_dirty,
                        field_response.lost_focus(),
                        field_response.has_focus(),
                        ui.input(|input| input.key_pressed(Key::Enter)),
                    ) {
                        self.manual_field_path_dirty = false;
                        self.refresh_selected_value();
                    }
                    if !self.node_selector.is_empty() {
                        ui.label("no snapshot paths available; enter the field path manually");
                    }
                }

                if self.available_layers.is_empty() {
                    ui.add(TextEdit::singleline(&mut self.target_layer).hint_text("target layer"));
                } else {
                    ComboBox::from_id_salt(ui.id().with("parameter-layer"))
                        .selected_text(self.target_layer.as_str())
                        .show_ui(ui, |ui| {
                            for layer in &self.available_layers {
                                ui.selectable_value(&mut self.target_layer, layer.clone(), layer);
                            }
                        });
                }

                if ui.button("Refresh").clicked() {
                    self.refresh_node_context();
                    self.refresh_selected_value();
                }
            });

            if let Some(metadata) = &self.field_metadata {
                ui.label(format_field_metadata(metadata));
            }

            ui.horizontal(|ui| {
                let can_mutate = !self.node_selector.is_empty()
                    && !self.field_path.is_empty()
                    && self.current_revision.is_some()
                    && !self.target_layer.is_empty();
                ui.add_enabled_ui(can_mutate, |ui| {
                    if ui.button("Commit").clicked() {
                        self.commit();
                    }
                    if ui.button("Reset layer").clicked() {
                        self.reset_scope();
                    }
                });

                if let Some(revision) = self.current_revision {
                    ui.label(format!("revision {revision}"));
                }
                if let Some(layer) = &self.effective_source_layer {
                    ui.label(format!("source {layer}"));
                }
                if let Some(config_key) = &self.config_key {
                    ui.label(format!("parameter key {config_key}"));
                }
            });

            ScrollArea::vertical().show(ui, |ui| {
                ui.add(
                    TextEdit::multiline(&mut self.parameter_value)
                        .code_editor()
                        .desired_width(f32::INFINITY),
                );
            });

            if let Some(status) = &self.status_message {
                ui.label(status);
            }
        })
        .response
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        ParameterFieldMetadata, flatten_parameter_paths, format_field_metadata,
        should_fetch_manual_field_path,
    };

    #[test]
    fn flatten_parameter_paths_returns_dot_notation_for_json_object_paths() {
        let value = json!({
            "enabled": true,
            "nested": {
                "count": 1,
                "inner": {
                    "gain": 0.5
                }
            },
            "items": [1, 2]
        });

        assert_eq!(
            flatten_parameter_paths(&value),
            vec![
                "enabled",
                "items",
                "nested",
                "nested.count",
                "nested.inner",
                "nested.inner.gain"
            ]
        );
    }

    #[test]
    fn should_fetch_manual_field_path_only_after_submission_or_focus_loss() {
        assert!(!should_fetch_manual_field_path(true, false, false, false));
        assert!(should_fetch_manual_field_path(true, true, false, false));
        assert!(should_fetch_manual_field_path(true, false, true, true));
        assert!(!should_fetch_manual_field_path(false, true, false, false));
        assert!(!should_fetch_manual_field_path(true, false, false, true));
    }

    #[test]
    fn format_field_metadata_excludes_schema_hash() {
        let metadata = ParameterFieldMetadata {
            type_name: "json".to_string(),
            writable: true,
            effective_source_layer: "robot.toml".to_string(),
        };

        assert_eq!(
            format_field_metadata(&metadata),
            "type=json writable=true source=robot.toml"
        );
    }
}
