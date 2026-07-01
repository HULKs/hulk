use std::sync::{Arc, Mutex};

use color_eyre::{Result, eyre::Context as _};
use ros_z::{context::ContextBuilder, prelude::*};
use ros_z_debug::{TopicObserver, TopicObserverOptions};
use tokio::runtime::Handle;
use uuid::Uuid;

pub struct RobotBackend {
    runtime_handle: Handle,
    context: Arc<Context>,
    _node: Arc<Node>,
    observer: TopicObserver,
    namespace: Mutex<String>,
}

impl RobotBackend {
    pub async fn new(
        runtime_handle: Handle,
        router: Option<String>,
        namespace: String,
    ) -> Result<Self> {
        let mut builder = ContextBuilder::default();
        if let Some(router) = router {
            builder = builder
                .with_router_endpoint(router)
                .wrap_err("failed to configure ROS-Z router endpoint")?;
        }

        let context = Arc::new(
            builder
                .build()
                .await
                .wrap_err("failed to build ROS-Z context")?,
        );
        let node_name = twix_node_name();
        let node = Arc::new(
            context
                .create_node(node_name)
                .with_namespace("/_twix")
                .build()
                .await
                .wrap_err("failed to create Twix ROS-Z node")?,
        );
        let options = TopicObserverOptions::with_namespace(namespace.clone())
            .wrap_err("failed to configure initial Twix namespace")?;
        let observer = TopicObserver::new(Arc::clone(&node), options);

        Ok(Self {
            runtime_handle,
            context,
            _node: node,
            observer,
            namespace: Mutex::new(namespace),
        })
    }

    pub fn runtime_handle(&self) -> &Handle {
        &self.runtime_handle
    }

    pub fn observer(&self) -> &TopicObserver {
        &self.observer
    }

    pub fn namespace(&self) -> String {
        self.namespace
            .lock()
            .expect("namespace mutex should not be poisoned")
            .clone()
    }

    pub fn set_namespace(&self, namespace: String) -> Result<()> {
        self.observer
            .set_namespace(namespace.clone())
            .wrap_err("failed to set Twix target namespace")?;
        *self
            .namespace
            .lock()
            .expect("namespace mutex should not be poisoned") = namespace;
        Ok(())
    }
}

impl Drop for RobotBackend {
    fn drop(&mut self) {
        if let Err(error) = self.context.shutdown() {
            log::error!("failed to shut down ROS-Z context: {error:#}");
        }
    }
}

fn twix_node_name() -> String {
    let host = std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown-host".to_string());
    let host = sanitize_node_component(&host);
    let id = Uuid::new_v4().simple().to_string();
    let short_id = &id[..8];
    format!("twix_{short_id}_{host}")
}

fn sanitize_node_component(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' || character == '-' {
                character
            } else {
                '_'
            }
        })
        .collect();

    if sanitized.is_empty() {
        "unknown-host".to_string()
    } else {
        sanitized
    }
}
