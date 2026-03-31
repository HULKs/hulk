use std::sync::Arc;

use eframe::egui::{Response, Ui, Widget};
use types::behavior_tree::NodeTrace;

use crate::{
    panel::{Panel, PanelCreationContext},
    robot::Robot,
    value_buffer::BufferHandle,
};

pub struct BehaviorTreePanel {
    buffer: BufferHandle<Option<NodeTrace>>,
}

impl<'a> Panel<'a> for BehaviorTreePanel {
    const NAME: &'static str = "Behavior Tree";

    fn new(context: PanelCreationContext) -> Self {
        Self {
            buffer: context.robot.subscribe_value("WorldState.additional_outputs.behavior_trace"),
        }
    }
}

impl Widget for &mut BehaviorTreePanel {
    fn ui(self, ui: &mut Ui) -> Response {
        match self.buffer.get_last_value() {
            Ok(Some(trace)) => ui.label(format!("{trace:#?}")),
            _ => ui.label("No data"),
        };

        ui.response()
    }
}
