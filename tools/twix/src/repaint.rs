use std::sync::Arc;

use eframe::egui::Context;
use ros_z_debug::{
    DynamicTopicObservation, TopicObservation, TopicObservationUpdateClosed,
    TopicObservationUpdateReceiver,
};
use tokio::{runtime::Handle, task::JoinHandle};

use crate::{
    backend::RobotBackend,
    panel::{PanelCreationContext, PanelUiContext},
};

pub trait ObservationContext {
    fn backend(&self) -> &Arc<RobotBackend>;
    fn egui_context(&self) -> Context;
}

impl ObservationContext for PanelCreationContext<'_> {
    fn backend(&self) -> &Arc<RobotBackend> {
        &self.backend
    }

    fn egui_context(&self) -> Context {
        self.egui_context.clone()
    }
}

impl ObservationContext for PanelUiContext<'_> {
    fn backend(&self) -> &Arc<RobotBackend> {
        self.backend
    }

    fn egui_context(&self) -> Context {
        self.egui_context.clone()
    }
}

pub struct ObservationRepaint {
    task: JoinHandle<()>,
}

impl ObservationRepaint {
    fn watch<T: Send + Sync + 'static>(
        context: &impl ObservationContext,
        mut updates: tokio::sync::watch::Receiver<T>,
    ) -> Self {
        let egui = context.egui_context();
        let task = context.backend().runtime_handle().spawn(async move {
            while updates.changed().await.is_ok() {
                egui.request_repaint();
            }
        });
        Self { task }
    }
    fn spawn(
        egui_context: Context,
        runtime_handle: &Handle,
        updates: Result<TopicObservationUpdateReceiver, TopicObservationUpdateClosed>,
    ) -> Self {
        let task = runtime_handle.spawn(async move {
            let Ok(mut updates) = updates else {
                egui_context.request_repaint();
                return;
            };

            loop {
                match updates.recv().await {
                    Ok(_) => egui_context.request_repaint(),
                    Err(_) => {
                        egui_context.request_repaint();
                        break;
                    }
                }
            }
        });

        Self { task }
    }
}

impl Drop for ObservationRepaint {
    fn drop(&mut self) {
        self.task.abort();
    }
}

pub trait RepaintOnUpdates {
    fn repaint_on_updates<C>(&self, context: &C) -> ObservationRepaint
    where
        C: ObservationContext;
}

impl RepaintOnUpdates for ros_z_debug::query::PlanObservation {
    fn repaint_on_updates<C: ObservationContext>(&self, context: &C) -> ObservationRepaint {
        ObservationRepaint::watch(context, self.subscribe_updates())
    }
}

impl RepaintOnUpdates for DynamicTopicObservation {
    fn repaint_on_updates<C>(&self, context: &C) -> ObservationRepaint
    where
        C: ObservationContext,
    {
        ObservationRepaint::spawn(
            context.egui_context(),
            context.backend().runtime_handle(),
            self.subscribe_updates(),
        )
    }
}

impl<T> RepaintOnUpdates for TopicObservation<T> {
    fn repaint_on_updates<C: ObservationContext>(&self, context: &C) -> ObservationRepaint {
        ObservationRepaint::spawn(
            context.egui_context(),
            context.backend().runtime_handle(),
            self.subscribe_updates(),
        )
    }
}
