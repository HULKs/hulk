use crate::model::StreamId;
use eframe::{egui, Storage};
use egui_dock::{DockState, NodeIndex};
use tracing::warn;

use super::{
    panels, ParameterPanelTab, PersistedUiState, TextPanelTab, ViewerApp, ViewerTab,
    STORAGE_KEY_DOCK_STATE, STORAGE_KEY_UI_STATE,
};

pub(super) struct ViewerTabHost<'a> {
    pub(super) app: &'a mut ViewerApp,
    pub(super) text_panel_count: usize,
}

impl egui_dock::TabViewer for ViewerTabHost<'_> {
    type Tab = ViewerTab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.title_label().into()
    }

    fn id(&mut self, tab: &mut Self::Tab) -> egui::Id {
        tab.dock_id()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        match tab {
            ViewerTab::Discovery => panels::draw_discovery_panel(self.app, ui),
            ViewerTab::Timeline => panels::draw_timeline_panel(self.app, ui),
            ViewerTab::Text(stream) => panels::draw_text_panel(self.app, ui, stream),
            ViewerTab::Parameters(panel) => panels::draw_parameters_panel(self.app, ui, panel),
        }
    }

    fn is_closeable(&self, tab: &Self::Tab) -> bool {
        tab.is_closeable(self.text_panel_count)
    }
}

pub(super) fn ensure_stream_tab_exists(
    dock_state: &mut DockState<ViewerTab>,
    default_stream: TextPanelTab,
) {
    let has_stream = dock_state
        .iter_all_tabs()
        .any(|(_, tab)| matches!(tab, ViewerTab::Text(_)));
    if !has_stream {
        dock_state.push_to_focused_leaf(ViewerTab::Text(default_stream));
    }
}

pub(super) fn ensure_timeline_tab_exists(dock_state: &mut DockState<ViewerTab>) {
    let has_timeline = dock_state
        .iter_all_tabs()
        .any(|(_, tab)| matches!(tab, ViewerTab::Timeline));
    if !has_timeline {
        dock_state.push_to_focused_leaf(ViewerTab::Timeline);
    }
}

pub(super) fn apply_overrides_to_primary_text_panel(
    dock_state: &mut DockState<ViewerTab>,
    source_expression: Option<&str>,
) {
    for (_, tab) in dock_state.iter_all_tabs_mut() {
        if let ViewerTab::Text(stream) = tab {
            if let Some(source_expression) = source_expression {
                stream.source_expression = source_expression.to_string();
            }
            return;
        }
    }
}

pub(super) fn highest_stream_id(dock_state: &DockState<ViewerTab>) -> StreamId {
    dock_state
        .iter_all_tabs()
        .filter_map(|(_, tab)| match tab {
            ViewerTab::Text(stream) => Some(stream.id),
            _ => None,
        })
        .max()
        .unwrap_or(0)
}

pub(super) fn highest_parameter_panel_id(dock_state: &DockState<ViewerTab>) -> u64 {
    dock_state
        .iter_all_tabs()
        .filter_map(|(_, tab)| match tab {
            ViewerTab::Parameters(panel) => Some(panel.id),
            _ => None,
        })
        .max()
        .unwrap_or(0)
}

pub(super) fn initial_dock_state(
    default_stream: TextPanelTab,
    default_parameter_panel: ParameterPanelTab,
) -> DockState<ViewerTab> {
    let mut dock_state = DockState::new(vec![ViewerTab::Text(default_stream)]);
    let [stream_leaf, _] = dock_state.main_surface_mut().split_left(
        NodeIndex::root(),
        0.72,
        vec![ViewerTab::Discovery],
    );
    let [stream_leaf, _] =
        dock_state
            .main_surface_mut()
            .split_below(stream_leaf, 0.85, vec![ViewerTab::Timeline]);
    let _ = dock_state.main_surface_mut().split_right(
        stream_leaf,
        0.78,
        vec![ViewerTab::Parameters(default_parameter_panel)],
    );
    dock_state
}

pub(super) fn load_persisted_dock_state(
    storage: Option<&dyn Storage>,
) -> Option<DockState<ViewerTab>> {
    let storage = storage?;
    let raw = storage.get_string(STORAGE_KEY_DOCK_STATE)?;
    match serde_json::from_str::<DockState<ViewerTab>>(&raw) {
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
