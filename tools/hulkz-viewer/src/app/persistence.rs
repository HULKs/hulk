use eframe::Storage;
use egui_dock::DockState;
use tracing::warn;

use super::{
    state::{PersistedUiState, ViewerApp},
    workspace_panel::WorkspacePanel,
};

pub(super) const STORAGE_KEY_DOCK_STATE: &str = "hulkz_viewer/workspace_dock_state_v7";
pub(super) const STORAGE_KEY_UI_STATE: &str = "hulkz_viewer/ui_state_v6";

pub(super) fn load_persisted_dock_state(
    storage: Option<&dyn Storage>,
) -> Option<DockState<WorkspacePanel>> {
    let storage = storage?;
    let raw = storage.get_string(STORAGE_KEY_DOCK_STATE)?;
    match serde_json::from_str::<DockState<WorkspacePanel>>(&raw) {
        Ok(state) => Some(state),
        Err(error) => {
            warn!(?error, "failed to deserialize persisted dock state");
            None
        }
    }
}

pub(super) fn load_persisted_ui_state(storage: Option<&dyn Storage>) -> Option<PersistedUiState> {
    let storage = storage?;
    let raw = storage.get_string(STORAGE_KEY_UI_STATE)?;
    match serde_json::from_str::<PersistedUiState>(&raw) {
        Ok(state) => Some(state),
        Err(error) => {
            warn!(?error, "failed to deserialize persisted ui state");
            None
        }
    }
}

pub(super) fn save_persisted_state(app: &ViewerApp, storage: &mut dyn Storage) {
    match serde_json::to_string(&app.workspace.dock_state) {
        Ok(json) => storage.set_string(STORAGE_KEY_DOCK_STATE, json),
        Err(error) => {
            warn!(?error, "failed to serialize dock state");
            return;
        }
    }

    let ui_state = PersistedUiState {
        ingest_enabled: app.ui.ingest_enabled,
        follow_live: app.ui.follow_live,
        next_stream_id: app.workspace.next_stream_id,
        next_parameter_panel_id: app.workspace.next_parameter_panel_id,
        default_namespace: app.ui.default_namespace.clone(),
        show_discovery: app.shell.show_discovery,
        show_timeline: app.shell.show_timeline,
    };

    match serde_json::to_string(&ui_state) {
        Ok(json) => storage.set_string(STORAGE_KEY_UI_STATE, json),
        Err(error) => warn!(?error, "failed to serialize ui state"),
    }
}
