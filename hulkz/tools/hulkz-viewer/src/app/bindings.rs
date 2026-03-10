use std::collections::BTreeSet;

use crate::protocol::{ParameterReference, WorkerCommand};

use super::{
    state::{StreamRuntimeState, ViewerApp},
    workspace_panel::WorkspacePanel,
    workspace_panel_kind::WorkspacePanelKind,
    workspace_panels::{
        NamespaceSelection, ParametersWorkspacePanelState, TextWorkspacePanelState,
    },
};

impl ViewerApp {
    pub(super) fn update_discovery_namespace(&mut self) {
        self.send_command(WorkerCommand::SetDiscoveryNamespace(
            self.ui.default_namespace.trim().to_string(),
        ));
    }

    fn apply_panel_binding(&mut self, panel: &TextWorkspacePanelState) {
        let stream_id = panel.id;
        if let Some(request) = panel.binding_request(&self.ui.default_namespace) {
            self.send_command(WorkerCommand::BindStream { stream_id, request });
        } else {
            self.send_command(WorkerCommand::RemoveStream { stream_id });
            if let Some(state) = self.workspace.stream_states.get_mut(&stream_id) {
                state.source_label = "unbound (set namespace/path)".to_string();
            }
        }
    }

    pub(super) fn reconcile_text_panels(&mut self) {
        let panels = self.workspace_text_panels();
        let panel_ids = panels.iter().map(|panel| panel.id).collect::<BTreeSet<_>>();

        for stream_id in self
            .workspace
            .stream_states
            .keys()
            .copied()
            .collect::<Vec<_>>()
            .into_iter()
            .filter(|stream_id| !panel_ids.contains(stream_id))
            .collect::<Vec<_>>()
        {
            self.workspace.stream_states.remove(&stream_id);
        }

        for (stream_id, previous_request) in self
            .workspace
            .binding_cache
            .clone()
            .into_iter()
            .filter(|(stream_id, _)| !panel_ids.contains(stream_id))
            .collect::<Vec<_>>()
        {
            if previous_request.is_some() {
                self.send_command(WorkerCommand::RemoveStream { stream_id });
            }
            self.unbind_stream_lane(stream_id);
            self.workspace.binding_cache.remove(&stream_id);
        }

        for panel in panels {
            self.workspace
                .stream_states
                .entry(panel.id)
                .or_insert_with(|| StreamRuntimeState {
                    source_label: "unbound".to_string(),
                    ..StreamRuntimeState::default()
                });

            let desired_request = panel.binding_request(&self.ui.default_namespace);
            let previous_request = self.workspace.binding_cache.get(&panel.id);
            if previous_request != Some(&desired_request) {
                self.apply_panel_binding(&panel);
                if desired_request.is_none() {
                    self.unbind_stream_lane(panel.id);
                }
                self.workspace
                    .binding_cache
                    .insert(panel.id, desired_request);
            }
        }

        self.evict_inactive_lanes_if_needed();
    }

    fn workspace_text_panels(&self) -> Vec<TextWorkspacePanelState> {
        self.workspace
            .dock_state
            .iter_all_tabs()
            .filter_map(|(_, tab)| match tab {
                WorkspacePanel::Text(stream) => Some(stream.clone()),
                WorkspacePanel::Parameters(_) => None,
            })
            .collect()
    }

    pub(super) fn create_text_workspace_panel(&mut self) {
        self.open_text_panel(
            NamespaceSelection::FollowDefault,
            self.config.source_expression.clone(),
        );
    }

    pub(super) fn open_workspace_panel_kind(&mut self, kind: WorkspacePanelKind) {
        match kind {
            WorkspacePanelKind::Text => self.create_text_workspace_panel(),
            WorkspacePanelKind::Parameters => self.create_parameter_panel(),
        }
    }

    pub(super) fn open_text_panel(
        &mut self,
        namespace_selection: NamespaceSelection,
        path_expression: String,
    ) {
        let stream_id = self.workspace.next_stream_id;
        self.workspace.next_stream_id = self.workspace.next_stream_id.saturating_add(1);

        let mut panel = TextWorkspacePanelState::new(stream_id, path_expression);
        panel.namespace_selection = namespace_selection;

        self.workspace
            .dock_state
            .push_to_focused_leaf(WorkspacePanel::Text(panel.clone()));
        self.workspace.stream_states.insert(
            stream_id,
            StreamRuntimeState {
                source_label: "unbound".to_string(),
                ..StreamRuntimeState::default()
            },
        );
    }

    fn create_parameter_panel(&mut self) {
        let panel_id = self.workspace.next_parameter_panel_id;
        self.workspace.next_parameter_panel_id =
            self.workspace.next_parameter_panel_id.saturating_add(1);
        self.workspace
            .dock_state
            .push_to_focused_leaf(WorkspacePanel::Parameters(
                ParametersWorkspacePanelState::new(panel_id),
            ));
    }

    pub(super) fn set_default_namespace(&mut self) {
        let namespace = self.ui.default_namespace_input.trim().to_string();
        self.ui.default_namespace_input = namespace;
        if self.ui.default_namespace == self.ui.default_namespace_input {
            return;
        }
        self.ui.default_namespace = self.ui.default_namespace_input.clone();
        self.update_discovery_namespace();
    }

    pub(super) fn parameter_node_candidates(
        &self,
        panel: &ParametersWorkspacePanelState,
    ) -> Vec<String> {
        let Some(namespace) = panel.effective_namespace(&self.ui.default_namespace) else {
            return Vec::new();
        };
        let namespace_filter = namespace.as_str();
        let path_filter = panel.path_input.trim();
        let mut candidates = BTreeSet::new();
        for parameter in &self.discovery.parameters {
            if parameter.namespace != namespace_filter {
                continue;
            }
            if !path_filter.is_empty() && parameter.path_expression != path_filter {
                continue;
            }
            let node = parameter.node.trim();
            if !node.is_empty() {
                candidates.insert(node.to_string());
            }
        }
        candidates.into_iter().collect()
    }

    pub(super) fn parameter_path_candidates(
        &self,
        panel: &ParametersWorkspacePanelState,
    ) -> Vec<String> {
        let Some(namespace) = panel.effective_namespace(&self.ui.default_namespace) else {
            return Vec::new();
        };
        let namespace_filter = namespace.as_str();
        let node_filter = panel.node_input.trim();
        let mut candidates = BTreeSet::new();
        for parameter in &self.discovery.parameters {
            if parameter.namespace != namespace_filter {
                continue;
            }
            if !node_filter.is_empty() && parameter.node != node_filter {
                continue;
            }
            let path = parameter.path_expression.trim();
            if !path.is_empty() {
                candidates.insert(path.to_string());
            }
        }
        candidates.into_iter().collect()
    }

    pub(super) fn parameter_reference_from_inputs(
        &self,
        panel: &ParametersWorkspacePanelState,
    ) -> Result<ParameterReference, String> {
        let Some(namespace) = panel.effective_namespace(&self.ui.default_namespace) else {
            return Err("Set a default namespace first.".to_string());
        };
        let namespace_filter = namespace.as_str();

        let path_expression = panel.path_input.trim();
        if path_expression.is_empty() {
            return Err("Enter a parameter path.".to_string());
        }

        let mut node = panel.node_input.trim().to_string();
        if node.is_empty() {
            if let Some(private_node) = super::private_node_from_expression(path_expression) {
                node = private_node.to_string();
            } else {
                let mut nodes = self
                    .discovery
                    .parameters
                    .iter()
                    .filter(|parameter| {
                        parameter.namespace == namespace_filter
                            && parameter.path_expression.trim() == path_expression
                    })
                    .map(|parameter| parameter.node.trim())
                    .filter(|node| !node.is_empty())
                    .collect::<BTreeSet<_>>();
                if nodes.len() == 1 {
                    node = nodes.pop_first().unwrap_or_default().to_string();
                } else if nodes.len() > 1 {
                    return Err("Parameter exists on multiple nodes. Pick a node.".to_string());
                }
            }
        }

        if node.is_empty() {
            return Err("Enter a node (or use ~node/path).".to_string());
        }

        Ok(ParameterReference {
            namespace,
            node,
            path_expression: path_expression.to_string(),
        })
    }

    pub(super) fn source_path_candidates(&self, stream: &TextWorkspacePanelState) -> Vec<String> {
        let effective_namespace = stream.effective_namespace(&self.ui.default_namespace);

        let mut candidates = BTreeSet::new();
        for publisher in &self.discovery.publishers {
            if let Some(namespace) = effective_namespace.as_deref() {
                if publisher.namespace != namespace {
                    continue;
                }
            }
            let path = publisher.path_expression.trim();
            if !path.is_empty() {
                candidates.insert(path.to_string());
            }
        }
        candidates.into_iter().collect()
    }

    pub(super) fn namespace_candidates(&self) -> Vec<String> {
        self.discovery
            .sessions
            .iter()
            .map(|session| session.namespace.trim())
            .filter(|namespace| !namespace.is_empty())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(str::to_string)
            .collect()
    }
}
