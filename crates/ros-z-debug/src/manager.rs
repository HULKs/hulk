use std::{marker::PhantomData, sync::Arc};

use ros_z::node::Node;

use crate::{JsonRenderPolicy, RetentionPolicy};

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ManagerOptions {
    pub namespace: String,
}

impl Default for ManagerOptions {
    fn default() -> Self {
        Self {
            namespace: "/".to_string(),
        }
    }
}

pub struct SubscriptionManager {
    node: Arc<Node>,
    options: ManagerOptions,
}

impl SubscriptionManager {
    pub fn new(node: Arc<Node>, options: ManagerOptions) -> Self {
        Self { node, options }
    }

    pub fn subscribe_typed<T>(&self, topic: impl Into<String>) -> TypedSubscriptionBuilder<'_, T> {
        TypedSubscriptionBuilder {
            manager: self,
            topic: topic.into(),
            retention: RetentionPolicy::LatestOnly,
            value: PhantomData,
        }
    }

    pub fn subscribe_dynamic(&self, topic: impl Into<String>) -> DynamicSubscriptionBuilder<'_> {
        DynamicSubscriptionBuilder {
            manager: self,
            topic: topic.into(),
            retention: RetentionPolicy::LatestOnly,
            json: None,
        }
    }

    pub fn node(&self) -> &Arc<Node> {
        &self.node
    }

    pub fn namespace(&self) -> &str {
        &self.options.namespace
    }
}

pub struct TypedSubscriptionBuilder<'a, T> {
    pub(crate) manager: &'a SubscriptionManager,
    pub(crate) topic: String,
    pub(crate) retention: RetentionPolicy,
    value: PhantomData<T>,
}

impl<T> TypedSubscriptionBuilder<'_, T> {
    pub fn retention(mut self, retention: RetentionPolicy) -> Self {
        self.retention = retention;
        self
    }

    pub fn manager(&self) -> &SubscriptionManager {
        self.manager
    }

    pub fn topic(&self) -> &str {
        &self.topic
    }

    pub fn retention_policy(&self) -> RetentionPolicy {
        self.retention
    }
}

pub struct DynamicSubscriptionBuilder<'a> {
    pub(crate) manager: &'a SubscriptionManager,
    pub(crate) topic: String,
    pub(crate) retention: RetentionPolicy,
    pub(crate) json: Option<JsonRenderPolicy>,
}

impl DynamicSubscriptionBuilder<'_> {
    pub fn retention(mut self, retention: RetentionPolicy) -> Self {
        self.retention = retention;
        self
    }

    pub fn json(mut self, policy: JsonRenderPolicy) -> Self {
        self.json = Some(policy);
        self
    }

    pub fn manager(&self) -> &SubscriptionManager {
        self.manager
    }

    pub fn topic(&self) -> &str {
        &self.topic
    }

    pub fn retention_policy(&self) -> RetentionPolicy {
        self.retention
    }

    pub fn json_policy(&self) -> Option<JsonRenderPolicy> {
        self.json
    }
}
