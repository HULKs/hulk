use eframe::egui;
use serde::{Deserialize, Serialize};

use super::workspace_panels::{ParametersWorkspacePanelState, TextWorkspacePanelState};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum WorkspacePanel {
    #[serde(alias = "TextStreamPanel")]
    Text(TextWorkspacePanelState),
    Parameters(ParametersWorkspacePanelState),
}

impl WorkspacePanel {
    pub(super) fn title_label(&self) -> &'static str {
        match self {
            WorkspacePanel::Text(_) => "Text",
            WorkspacePanel::Parameters(_) => "Parameters",
        }
    }

    pub(super) fn dock_id(&self) -> egui::Id {
        match self {
            WorkspacePanel::Text(panel) => egui::Id::new(("workspace_panel_text", panel.id)),
            WorkspacePanel::Parameters(panel) => {
                egui::Id::new(("workspace_panel_parameters", panel.id))
            }
        }
    }

    pub(super) fn is_closeable(&self, text_panel_count: usize) -> bool {
        match self {
            WorkspacePanel::Text(_) => text_panel_count > 1,
            WorkspacePanel::Parameters(_) => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ParametersWorkspacePanelState, TextWorkspacePanelState, WorkspacePanel};

    #[test]
    fn title_and_closeability_match_expected_behavior() {
        let text = WorkspacePanel::Text(TextWorkspacePanelState::new(0, "odometry".to_string()));
        let parameters = WorkspacePanel::Parameters(ParametersWorkspacePanelState::new(0));

        assert_eq!(text.title_label(), "Text");
        assert_eq!(parameters.title_label(), "Parameters");
        assert!(!text.is_closeable(1));
        assert!(text.is_closeable(2));
        assert!(parameters.is_closeable(1));
    }

    #[test]
    fn serde_alias_supports_legacy_text_variant_name() {
        let legacy_json = r#"{
            "TextStreamPanel": {
                "id": 7,
                "namespace_selection": "FollowDefault",
                "source_expression": "odometry"
            }
        }"#;

        let panel =
            serde_json::from_str::<WorkspacePanel>(legacy_json).expect("legacy alias should load");
        assert!(matches!(panel, WorkspacePanel::Text(_)));
    }
}
