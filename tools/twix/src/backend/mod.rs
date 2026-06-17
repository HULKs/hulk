pub mod catalog;
pub mod json_buffer;
pub mod subscription;
pub mod topic;

use std::sync::Arc;

use color_eyre::{Result, eyre::eyre};
use eframe::egui::Context as EguiContext;
use log::error;
use parking_lot::Mutex;
use ros_z::{context::ContextBuilder, node::Node};
use serde_json::Value;
use tokio::{
    runtime::{Builder, Runtime},
    sync::watch,
};

use crate::{
    backend::catalog::TopicCatalog,
    value_buffer::{BufferHandle, BufferHistory},
};

pub struct TwixBackend {
    node: Arc<Node>,
    target_namespace_sender: watch::Sender<String>,
    topic_catalog: Arc<Mutex<Arc<TopicCatalog>>>,
    egui_context: EguiContext,
    runtime: Runtime,
}

impl TwixBackend {
    pub fn validate_router_endpoint(router_endpoint: impl AsRef<str>) -> Result<()> {
        let router_endpoint = router_endpoint.as_ref();
        if router_endpoint.trim().is_empty() {
            return Err(eyre!("router endpoint must not be empty"));
        }

        ContextBuilder::default().with_router_endpoint(router_endpoint.to_string())?;
        Ok(())
    }

    pub fn new(
        router_endpoint: impl Into<String>,
        target_namespace: impl AsRef<str>,
        egui_context: EguiContext,
    ) -> Result<Self> {
        let target_namespace = topic::normalize_namespace(target_namespace.as_ref())?;
        let router_endpoint = router_endpoint.into();
        Self::validate_router_endpoint(&router_endpoint)?;
        let runtime = Builder::new_multi_thread().enable_all().build()?;
        let node = runtime.block_on(async move {
            let context = ContextBuilder::default()
                .with_router_endpoint(router_endpoint)?
                .build()
                .await?;
            let node = context.create_node("twix").build().await?;
            Ok::<_, color_eyre::Report>(Arc::new(node))
        })?;
        let (target_namespace_sender, target_namespace_receiver) = watch::channel(target_namespace);
        let topic_catalog = Arc::new(Mutex::new(Arc::new(TopicCatalog::default())));
        let backend_egui_context = egui_context.clone();

        spawn_catalog_task(
            &runtime,
            node.clone(),
            target_namespace_receiver,
            topic_catalog.clone(),
            egui_context,
        );

        Ok(Self {
            node,
            target_namespace_sender,
            topic_catalog,
            egui_context: backend_egui_context,
            runtime,
        })
    }

    pub fn target_namespace(&self) -> String {
        self.target_namespace_sender.borrow().clone()
    }

    pub fn set_target_namespace(&self, target_namespace: impl AsRef<str>) -> Result<()> {
        let target_namespace = topic::normalize_namespace(target_namespace.as_ref())?;
        if self.target_namespace() != target_namespace {
            self.target_namespace_sender.send_replace(target_namespace);
        }
        Ok(())
    }

    pub fn topic_catalog(&self) -> Arc<TopicCatalog> {
        self.topic_catalog.lock().clone()
    }

    pub fn subscribe_json(
        &self,
        selector: impl Into<String>,
        history: BufferHistory,
    ) -> BufferHandle<Value> {
        json_buffer::subscribe_json(
            &self.runtime,
            self.node.clone(),
            self.target_namespace_sender.subscribe(),
            self.egui_context.clone(),
            selector,
            history,
        )
    }

    pub fn subscribe_changes_json(
        &self,
        selector: impl Into<String>,
    ) -> crate::change_buffer::ChangeBufferHandle<Value> {
        crate::change_buffer::spawn_json_change_buffer(
            &self.runtime,
            self.node.clone(),
            self.target_namespace_sender.subscribe(),
            self.egui_context.clone(),
            selector.into(),
        )
    }
}

fn spawn_catalog_task(
    runtime: &Runtime,
    node: Arc<Node>,
    mut target_namespace: watch::Receiver<String>,
    topic_catalog: Arc<Mutex<Arc<TopicCatalog>>>,
    egui_context: EguiContext,
) {
    let mut graph_changes = node.graph().subscribe_changes();
    runtime.spawn(async move {
        rebuild_topic_catalog(&node, &mut target_namespace, &topic_catalog, &egui_context);

        loop {
            tokio::select! {
                changed = graph_changes.changed() => {
                    if changed.is_err() {
                        break;
                    }
                }
                changed = target_namespace.changed() => {
                    if changed.is_err() {
                        break;
                    }
                }
            }

            rebuild_topic_catalog(&node, &mut target_namespace, &topic_catalog, &egui_context);
        }
    });
}

fn rebuild_topic_catalog(
    node: &Node,
    target_namespace: &mut watch::Receiver<String>,
    topic_catalog: &Mutex<Arc<TopicCatalog>>,
    egui_context: &EguiContext,
) {
    let target_namespace = target_namespace.borrow_and_update().clone();
    match catalog::build_topic_catalog(&target_namespace, &node.graph().view()) {
        Ok(catalog) => {
            *topic_catalog.lock() = Arc::new(catalog);
            egui_context.request_repaint();
        }
        Err(error) => error!("failed to rebuild Twix topic catalog: {error:#}"),
    }
}
