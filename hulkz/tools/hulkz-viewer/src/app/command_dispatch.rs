use tokio::sync::mpsc;
use tracing::{debug, warn};

use crate::protocol::WorkerCommand;

use super::{
    panel_api::{ShellPaneKind, UiIntent},
    state::ViewerApp,
    workspace_panel::WorkspacePanel,
};

impl ViewerApp {
    pub fn send_command(&mut self, command: WorkerCommand) {
        debug!(?command, "sending worker command");
        self.runtime.pending_commands.push_back(command);
    }

    pub fn run_pending_commands(&mut self) {
        loop {
            let Some(command) = self.runtime.pending_commands.pop_front() else {
                break;
            };
            match self.runtime.command_tx.try_send(command) {
                Ok(()) => {}
                Err(mpsc::error::TrySendError::Full(command)) => {
                    self.runtime.pending_commands.push_front(command);
                    break;
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    self.ui.last_error = Some("worker command channel is closed".to_string());
                    warn!("failed to send worker command: channel closed");
                    break;
                }
            }
        }
    }

    pub fn apply_ui_intents(&mut self, intents: Vec<UiIntent>) {
        for intent in intents {
            match intent {
                UiIntent::SetIngestEnabled(enabled) => {
                    self.ui.ingest_enabled = enabled;
                    self.send_command(WorkerCommand::SetIngestEnabled(enabled));
                }
                UiIntent::SetShellPaneVisible { pane, visible } => match pane {
                    ShellPaneKind::Discovery => self.shell.show_discovery = visible,
                    ShellPaneKind::Timeline => self.shell.show_timeline = visible,
                },
                UiIntent::OpenWorkspacePanel(kind) => self.open_workspace_panel_kind(kind),
                UiIntent::OpenTextWorkspacePanel {
                    namespace_selection,
                    path_expression,
                } => self.open_text_panel(namespace_selection, path_expression),
                UiIntent::SetDefaultNamespaceDraft(input) => {
                    self.ui.default_namespace_input = input;
                }
                UiIntent::SetDefaultNamespaceCommitted(input) => {
                    self.ui.default_namespace_input = input;
                    self.set_default_namespace();
                }
                UiIntent::BindOrRebindTextPanel {
                    panel_id,
                    namespace_selection,
                    source_expression,
                } => {
                    for (_, tab) in self.workspace.dock_state.iter_all_tabs_mut() {
                        if let WorkspacePanel::Text(panel) = tab {
                            if panel.id == panel_id {
                                panel.namespace_selection = namespace_selection.clone();
                                panel.source_expression = source_expression.clone();
                            }
                        }
                    }
                }
                UiIntent::ReadParameter { panel_id, target } => {
                    for (_, tab) in self.workspace.dock_state.iter_all_tabs_mut() {
                        if let WorkspacePanel::Parameters(panel) = tab {
                            if panel.id == panel_id {
                                panel.status = None;
                            }
                        }
                    }
                    self.send_command(WorkerCommand::ReadParameter(target));
                }
                UiIntent::ApplyParameter {
                    panel_id,
                    target,
                    value_json,
                } => {
                    for (_, tab) in self.workspace.dock_state.iter_all_tabs_mut() {
                        if let WorkspacePanel::Parameters(panel) = tab {
                            if panel.id == panel_id {
                                panel.status = None;
                            }
                        }
                    }
                    self.send_command(WorkerCommand::SetParameter { target, value_json });
                }
                UiIntent::TimelineSetFollowLive(follow_live) => {
                    if follow_live {
                        self.ui.follow_live = true;
                        self.jump_latest_internal(true);
                    } else {
                        if self.ui.follow_live {
                            self.freeze_timeline_window_at_current_range();
                        }
                        self.ui.follow_live = false;
                    }
                }
                UiIntent::TimelineJumpLatest => {
                    self.ui.follow_live = true;
                    self.jump_latest_internal(true);
                }
                UiIntent::TimelineSelectAnchor(anchor_ns) => {
                    self.mark_manual_timeline_navigation();
                    self.set_global_timeline_anchor_by_timestamp(anchor_ns, true);
                }
                UiIntent::TimelinePan(delta_fraction) => {
                    self.mark_manual_timeline_navigation();
                    self.apply_timeline_pan_fraction(delta_fraction);
                }
                UiIntent::TimelineZoom { factor, focus_ns } => {
                    self.mark_manual_timeline_navigation();
                    self.apply_timeline_zoom(factor, focus_ns);
                }
                UiIntent::TimelineLaneScroll(_delta) => {
                    // lane scroll is applied directly during draw because it depends on live canvas metrics
                }
            }
        }
    }
}
