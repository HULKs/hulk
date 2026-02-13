use crate::protocol::ParameterReference;

use super::{
    state::ViewerApp, workspace_panel_kind::WorkspacePanelKind,
    workspace_panels::NamespaceSelection,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellPaneKind {
    Discovery,
    Timeline,
}

#[derive(Debug, Clone)]
pub enum UiIntent {
    SetIngestEnabled(bool),
    SetShellPaneVisible {
        pane: ShellPaneKind,
        visible: bool,
    },
    OpenWorkspacePanel(WorkspacePanelKind),
    OpenTextWorkspacePanel {
        namespace_selection: NamespaceSelection,
        path_expression: String,
    },
    SetDefaultNamespaceDraft(String),
    SetDefaultNamespaceCommitted(String),
    BindOrRebindTextPanel {
        panel_id: u64,
        namespace_selection: NamespaceSelection,
        source_expression: String,
    },
    ReadParameter {
        panel_id: u64,
        target: ParameterReference,
    },
    ApplyParameter {
        panel_id: u64,
        target: ParameterReference,
        value_json: String,
    },
    TimelineSetFollowLive(bool),
    TimelineJumpLatest,
    TimelineSelectAnchor(u64),
    TimelinePan(f32),
    TimelineZoom {
        factor: f32,
        focus_ns: u64,
    },
    TimelineLaneScroll(f32),
}

pub struct PanelContext<'a> {
    app: &'a mut ViewerApp,
    intents: &'a mut Vec<UiIntent>,
}

impl<'a> PanelContext<'a> {
    pub fn new(app: &'a mut ViewerApp, intents: &'a mut Vec<UiIntent>) -> Self {
        Self { app, intents }
    }

    pub fn app(&self) -> &ViewerApp {
        self.app
    }

    pub fn app_mut(&mut self) -> &mut ViewerApp {
        self.app
    }

    pub fn emit(&mut self, intent: UiIntent) {
        self.intents.push(intent);
    }
}
