use eframe::egui::{Response, Ui, Widget};
use twix_legacy::behavior_tree::BehaviorTreeVisualizer;
use types::behavior_tree::NodeTrace;

use crate::{
    panel::{Panel, PanelCreationContext},
    value_buffer::BufferHandle,
};

pub struct BehaviorTreePanel {
    tree_layout_buffer: BufferHandle<Option<NodeTrace>>,
    trace_buffer: BufferHandle<Option<NodeTrace>>,
    visualizer: BehaviorTreeVisualizer,
}

impl<'a> Panel<'a> for BehaviorTreePanel {
    const NAME: &'static str = "Behavior Tree";

    fn new(context: PanelCreationContext) -> Self {
        Self {
            tree_layout_buffer: context
                .robot
                .subscribe_value("WorldState.additional_outputs.behavior.tree_layout"),
            trace_buffer: context
                .robot
                .subscribe_value("WorldState.additional_outputs.behavior.trace"),
            visualizer: BehaviorTreeVisualizer::default(),
        }
    }
}

impl Widget for &mut BehaviorTreePanel {
    fn ui(self, ui: &mut Ui) -> Response {
        let tree_layout = self
            .tree_layout_buffer
            .get_last_value()
            .ok()
            .flatten()
            .flatten();
        let trace = self.trace_buffer.get_last_value().ok().flatten().flatten();

        self.visualizer
            .show(ui, tree_layout.as_ref(), trace.as_ref())
    }
}
