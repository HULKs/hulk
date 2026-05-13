use std::sync::Arc;

use ros_z::{context::Context, context::ContextBuilder, node::Node};

use crate::{ManagerOptions, Result, SubscriptionManager};

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ConnectionConfig {
    pub endpoints: Vec<String>,
    pub namespace: String,
    pub node_name: String,
}

impl ConnectionConfig {
    pub fn new(endpoints: Vec<String>) -> Self {
        Self {
            endpoints,
            namespace: "/".to_string(),
            node_name: "ros_z_debug".to_string(),
        }
    }
}

struct ActiveConnection {
    manager: SubscriptionManager,
    context: Context,
}

pub struct ConnectionManager {
    config: ConnectionConfig,
    generation: u64,
    active: Option<ActiveConnection>,
}

impl ConnectionManager {
    pub fn new(config: ConnectionConfig) -> Self {
        Self {
            config,
            generation: 0,
            active: None,
        }
    }

    pub async fn connect(&mut self) -> Result<()> {
        self.disconnect().await?;

        let generation = self.generation + 1;
        let context = ContextBuilder::default()
            .with_mode("client")
            .with_namespace(&self.config.namespace)
            .with_connect_endpoints(self.config.endpoints.clone())
            .build()
            .await?;
        let node: Arc<Node> = Arc::new(context.create_node(&self.config.node_name).build().await?);
        let manager = SubscriptionManager::new(
            node,
            ManagerOptions {
                namespace: self.config.namespace.clone(),
            },
        );

        self.generation = generation;
        self.active = Some(ActiveConnection { manager, context });

        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        let Some(active) = self.active.take() else {
            return Ok(());
        };

        let ActiveConnection {
            context, manager, ..
        } = active;
        manager.close();
        drop(manager);
        context.shutdown()?;

        Ok(())
    }

    pub fn manager(&self) -> Option<&SubscriptionManager> {
        self.active.as_ref().map(|active| &active.manager)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_new_preserves_endpoints_and_uses_defaults() {
        let endpoints = vec![
            "tcp/127.0.0.1:7447".to_string(),
            "tcp/127.0.0.1:7448".to_string(),
        ];

        let config = ConnectionConfig::new(endpoints.clone());

        assert_eq!(config.endpoints, endpoints);
        assert_eq!(config.namespace, "/");
        assert_eq!(config.node_name, "ros_z_debug");
    }

    #[test]
    fn new_manager_starts_disconnected_with_generation_zero() {
        let manager = ConnectionManager::new(ConnectionConfig::new(Vec::new()));

        assert!(manager.manager().is_none());
        assert_eq!(manager.generation, 0);
        assert!(manager.active.is_none());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn disconnect_without_active_connection_is_noop() {
        let mut manager = ConnectionManager::new(ConnectionConfig::new(Vec::new()));

        manager.disconnect().await.unwrap();

        assert!(manager.manager().is_none());
        assert_eq!(manager.generation, 0);
        assert!(manager.active.is_none());
    }
}
