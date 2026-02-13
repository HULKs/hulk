mod bindings;
mod bootstrap;
mod command_dispatch;
mod event_ingest;
mod layout;
mod panel_api;
mod panel_prelude;
mod panes;
mod persistence;
mod shell;
mod state;
mod time_fmt;
mod timeline_state;
mod update_loop;
mod workspace;
mod workspace_panel;
mod workspace_panel_kind;
mod workspace_panels;

use std::time::Duration;

use tracing::info;

use crate::protocol::WorkerCommand;

pub use self::state::ViewerApp;
pub use self::state::ViewerStartupOverrides;
pub(crate) use self::state::{LaneRenderRow, TimelineRenderRange};
pub(crate) use self::time_fmt::format_timestamp;
pub(crate) use self::timeline_state::is_manual_timeline_navigation;
impl ViewerApp {
    fn initiate_shutdown(&mut self) {
        if self.runtime.shutdown_started {
            return;
        }
        self.runtime.shutdown_started = true;
        info!("shutting down viewer app");

        self.send_command(WorkerCommand::Shutdown);
        self.run_pending_commands();
        self.runtime.cancellation_token.cancel();

        if let Some(worker_task) = self.runtime.worker_task.take() {
            let _ = self.runtime.runtime.block_on(async {
                tokio::time::timeout(Duration::from_secs(2), worker_task).await
            });
        }
        info!("viewer shutdown sequence completed");
    }
}

impl Drop for ViewerApp {
    fn drop(&mut self) {
        self.initiate_shutdown();
    }
}

fn private_node_from_expression(path_expression: &str) -> Option<&str> {
    let remainder = path_expression.strip_prefix('~')?;
    let (node, _) = remainder.split_once('/')?;
    if node.is_empty() {
        None
    } else {
        Some(node)
    }
}
