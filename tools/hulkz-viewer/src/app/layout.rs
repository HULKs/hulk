use crate::model::StreamId;
use eframe::egui;
use egui_dock::{DockState, Node, NodeIndex, SurfaceIndex};

use super::{
    panels,
    state::{ParameterPanelTab, TextPanelTab, ViewerApp, ViewerTab},
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
            ViewerTab::Parameters(_) => None,
        })
        .max()
        .unwrap_or(0)
}

pub(super) fn highest_parameter_panel_id(dock_state: &DockState<ViewerTab>) -> u64 {
    dock_state
        .iter_all_tabs()
        .filter_map(|(_, tab)| match tab {
            ViewerTab::Parameters(panel) => Some(panel.id),
            ViewerTab::Text(_) => None,
        })
        .max()
        .unwrap_or(0)
}

pub(super) fn initial_dock_state(
    default_stream: TextPanelTab,
    default_parameter_panel: ParameterPanelTab,
) -> DockState<ViewerTab> {
    let mut dock_state = DockState::new(vec![ViewerTab::Text(default_stream)]);
    let _ = dock_state.main_surface_mut().split_right(
        NodeIndex::root(),
        0.78,
        vec![ViewerTab::Parameters(default_parameter_panel)],
    );
    dock_state
}

const MIN_SPLIT_FRACTION: f32 = 0.05;
const MAX_SPLIT_FRACTION: f32 = 0.95;

pub(super) fn sanitize_dock_splits(dock_state: &mut DockState<ViewerTab>) -> bool {
    let mut changed = false;
    let mut surface_index = 0usize;
    loop {
        let Some(surface) = dock_state.get_surface_mut(SurfaceIndex(surface_index)) else {
            break;
        };
        for node in surface.iter_nodes_mut() {
            if let Node::Vertical(split) | Node::Horizontal(split) = node {
                let original = split.fraction;
                let sanitized = if original.is_finite() {
                    original.clamp(MIN_SPLIT_FRACTION, MAX_SPLIT_FRACTION)
                } else {
                    0.5
                };
                if (sanitized - original).abs() > f32::EPSILON {
                    split.fraction = sanitized;
                    changed = true;
                }
            }
        }
        surface_index = surface_index.saturating_add(1);
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::{
        initial_dock_state, sanitize_dock_splits, ParameterPanelTab, TextPanelTab,
        MAX_SPLIT_FRACTION, MIN_SPLIT_FRACTION,
    };
    use crate::app::state::ViewerTab;
    use egui_dock::Node;

    #[test]
    fn sanitize_dock_splits_clamps_invalid_split_fractions() {
        let mut dock_state = initial_dock_state(
            TextPanelTab::new(0, "odometry".to_string()),
            ParameterPanelTab::new(0),
        );
        for node in dock_state.main_surface_mut().iter_mut() {
            if let Node::Vertical(split) | Node::Horizontal(split) = node {
                split.fraction = 0.0;
            }
        }

        let changed = sanitize_dock_splits(&mut dock_state);
        assert!(changed);
        for node in dock_state.main_surface().iter() {
            if let Node::Vertical(split) | Node::Horizontal(split) = node {
                assert!((MIN_SPLIT_FRACTION..=MAX_SPLIT_FRACTION).contains(&split.fraction));
            }
        }
    }

    #[test]
    fn sanitize_dock_splits_noop_for_valid_layout() {
        let mut dock_state = initial_dock_state(
            TextPanelTab::new(0, "odometry".to_string()),
            ParameterPanelTab::new(0),
        );
        assert!(!sanitize_dock_splits(&mut dock_state));
    }

    #[test]
    fn initial_layout_contains_workspace_tabs() {
        let dock_state = initial_dock_state(
            TextPanelTab::new(0, "odometry".to_string()),
            ParameterPanelTab::new(0),
        );
        let tabs = dock_state
            .iter_all_tabs()
            .map(|(_, tab)| tab)
            .collect::<Vec<_>>();
        assert!(tabs.iter().any(|tab| matches!(tab, ViewerTab::Text(_))));
        assert!(tabs
            .iter()
            .any(|tab| matches!(tab, ViewerTab::Parameters(_))));
    }
}
