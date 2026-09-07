use eframe::egui::Ui;
use ros_z::qos::{QosDurability, QosProfile};
use ros_z_debug::{ObservationPolicy, TopicObservation};
use twix_visualization::behavior_tree::BehaviorTreeVisualizer;
use types::behavior_tree::NodeTrace;

use crate::{
    panel::{Panel, PanelCreationContext, PanelUiContext},
    repaint::{ObservationRepaint, RepaintOnUpdates},
};

pub struct BehaviorTreePanel {
    tree_layout: TopicObservation<NodeTrace>,
    trace: TopicObservation<NodeTrace>,
    _tree_layout_repaint: ObservationRepaint,
    _trace_repaint: ObservationRepaint,
    visualizer: BehaviorTreeVisualizer,
}

impl Panel for BehaviorTreePanel {
    const STORAGE_ID: &'static str = "behavior_tree";
    const DISPLAY_NAME: &'static str = "Behavior Tree";

    fn new(context: PanelCreationContext<'_>) -> Self {
        let runtime_handle = context.backend.runtime_handle().clone();
        let _runtime_context = runtime_handle.enter();

        let tree_layout = context
            .backend
            .observer()
            .observe_typed("behavior/tree_layout")
            .expect("failed to construct behavior tree layout observer")
            .policy(
                ObservationPolicy::default().with_subscriber_qos(QosProfile {
                    durability: QosDurability::TransientLocal,
                    ..Default::default()
                }),
            )
            .spawn();
        let trace = context
            .backend
            .observer()
            .observe_typed("behavior/trace")
            .expect("failed to construct behavior trace observer")
            .spawn();
        let tree_layout_repaint = tree_layout.repaint_on_updates(&context);
        let trace_repaint = trace.repaint_on_updates(&context);

        Self {
            tree_layout,
            trace,
            _tree_layout_repaint: tree_layout_repaint,
            _trace_repaint: trace_repaint,
            visualizer: BehaviorTreeVisualizer::default(),
        }
    }

    fn ui(&mut self, ui: &mut Ui, _context: PanelUiContext<'_>) {
        let tree_layout = self.tree_layout.latest();
        let trace = self.trace.latest();

        self.visualizer.show(
            ui,
            tree_layout.as_deref().map(|record| &record.value),
            trace.as_deref().map(|record| &record.value),
        );
    }
}
