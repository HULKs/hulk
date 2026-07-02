use std::{
    net::IpAddr,
    sync::{Arc, Mutex},
};

use color_eyre::{
    Result,
    eyre::{Context as _, eyre},
};
use ros_z::{context::ContextBuilder, prelude::*};
use ros_z_debug::{TopicObserver, TopicObserverOptions};
use serde_json::json;
use tokio::{
    runtime::Handle,
    task::JoinHandle,
    time::{self, Duration},
};
use uuid::Uuid;

use crate::zenoh_router::HiddenZenohRouter;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendStatus {
    pub local_router_endpoint: String,
    pub direct_upstream_links: Vec<DirectUpstreamLinkStatus>,
}

impl BackendStatus {
    fn new(local_router_endpoint: String, upstream_endpoints: &[String]) -> Self {
        Self {
            local_router_endpoint,
            direct_upstream_links: upstream_endpoints
                .iter()
                .map(|endpoint| DirectUpstreamLinkStatus {
                    endpoint: endpoint.clone(),
                    state: DirectUpstreamLinkState::Unknown,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectUpstreamLinkStatus {
    pub endpoint: String,
    pub state: DirectUpstreamLinkState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectUpstreamLinkState {
    Connected,
    Disconnected,
    Unknown,
}

struct RobotBackendLive {
    upstream_status_task: JoinHandle<()>,
    _router: HiddenZenohRouter,
    context: Arc<Context>,
    _node: Arc<Node>,
    observer: TopicObserver,
}

pub struct RobotBackend {
    runtime_handle: Handle,
    live: RobotBackendLive,
    status: Arc<Mutex<BackendStatus>>,
    namespace: Mutex<String>,
}

impl RobotBackend {
    pub async fn new(
        runtime_handle: Handle,
        router_endpoints: Vec<String>,
        namespace: String,
    ) -> Result<Self> {
        let options = TopicObserverOptions::with_namespace(namespace.clone())
            .wrap_err("failed to configure initial Twix namespace")?;
        let (live, status) = RobotBackendLive::start(router_endpoints, options).await?;

        Ok(Self {
            runtime_handle,
            live,
            status,
            namespace: Mutex::new(namespace),
        })
    }

    pub fn runtime_handle(&self) -> &Handle {
        &self.runtime_handle
    }

    pub fn observer(&self) -> &TopicObserver {
        &self.live.observer
    }

    pub fn status(&self) -> BackendStatus {
        self.status
            .lock()
            .expect("backend status mutex should not be poisoned")
            .clone()
    }

    pub fn namespace(&self) -> String {
        self.namespace
            .lock()
            .expect("namespace mutex should not be poisoned")
            .clone()
    }

    pub fn set_namespace(&self, namespace: String) -> Result<()> {
        TopicObserverOptions::with_namespace(namespace.clone())
            .wrap_err("failed to validate Twix target namespace")?;
        self.live
            .observer
            .set_namespace(namespace.clone())
            .wrap_err("failed to set Twix target namespace")?;
        *self
            .namespace
            .lock()
            .expect("namespace mutex should not be poisoned") = namespace;
        Ok(())
    }
}

impl RobotBackendLive {
    async fn start(
        router_endpoints: Vec<String>,
        options: TopicObserverOptions,
    ) -> Result<(Self, Arc<Mutex<BackendStatus>>)> {
        let router = HiddenZenohRouter::start(&router_endpoints).await?;
        let local_endpoint = router.local_endpoint().to_string();
        let status = Arc::new(Mutex::new(BackendStatus::new(
            local_endpoint.clone(),
            &router_endpoints,
        )));

        let context = Arc::new(
            ContextBuilder::default()
                .with_router_endpoint(local_endpoint.clone())
                .wrap_err("failed to configure Twix ROS-Z context through hidden Zenoh router")?
                .with_json("connect/timeout_ms", json!(0))
                .with_json("connect/exit_on_failure", json!(false))
                .build()
                .await
                .wrap_err("failed to build ROS-Z context through hidden Zenoh router")?,
        );

        if context
            .session()
            .info()
            .routers_zid()
            .await
            .next()
            .is_none()
        {
            return Err(eyre!("Twix session does not see hidden Zenoh router"));
        }

        let node_name = twix_node_name();
        let node = Arc::new(
            context
                .create_node(node_name)
                .with_namespace("/_twix")
                .build()
                .await
                .wrap_err("failed to create Twix ROS-Z node")?,
        );
        let observer = TopicObserver::new(Arc::clone(&node), options);
        let upstream_status_task =
            spawn_upstream_status_task(router.session(), router_endpoints, Arc::clone(&status));

        Ok((
            Self {
                upstream_status_task,
                _router: router,
                context,
                _node: node,
                observer,
            },
            status,
        ))
    }
}

fn spawn_upstream_status_task(
    router_session: zenoh::Session,
    upstream_endpoints: Vec<String>,
    status: Arc<Mutex<BackendStatus>>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            refresh_upstream_status(&router_session, &upstream_endpoints, &status).await;
            time::sleep(Duration::from_secs(1)).await;
        }
    })
}

async fn refresh_upstream_status(
    router_session: &zenoh::Session,
    upstream_endpoints: &[String],
    status: &Arc<Mutex<BackendStatus>>,
) {
    let link_destinations: Vec<String> = router_session
        .info()
        .links()
        .await
        .map(|link| link.dst().to_string())
        .collect();
    let direct_upstream_links =
        direct_upstream_link_statuses(upstream_endpoints, &link_destinations);

    status
        .lock()
        .expect("backend status mutex should not be poisoned")
        .direct_upstream_links = direct_upstream_links;
}

fn direct_upstream_link_statuses(
    upstream_endpoints: &[String],
    link_destinations: &[String],
) -> Vec<DirectUpstreamLinkStatus> {
    upstream_endpoints
        .iter()
        .map(|endpoint| DirectUpstreamLinkStatus {
            endpoint: endpoint.clone(),
            state: direct_upstream_link_state(endpoint, link_destinations),
        })
        .collect()
}

fn direct_upstream_link_state(
    endpoint: &str,
    link_destinations: &[String],
) -> DirectUpstreamLinkState {
    if link_destinations
        .iter()
        .any(|destination| destination == endpoint)
    {
        DirectUpstreamLinkState::Connected
    } else if endpoint_has_literal_tcp_host(endpoint) {
        DirectUpstreamLinkState::Disconnected
    } else {
        DirectUpstreamLinkState::Unknown
    }
}

fn endpoint_has_literal_tcp_host(endpoint: &str) -> bool {
    let Some(address) = endpoint.strip_prefix("tcp/") else {
        return false;
    };
    let Some((host, port)) = address.rsplit_once(':') else {
        return false;
    };
    if port.parse::<u16>().is_err() {
        return false;
    }
    let host = host.trim_start_matches('[').trim_end_matches(']');
    host.parse::<IpAddr>().is_ok()
}

impl Drop for RobotBackendLive {
    fn drop(&mut self) {
        self.upstream_status_task.abort();

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

#[cfg(test)]
mod tests {
    use std::{net::TcpListener, time::Duration};

    use super::{
        BackendStatus, DirectUpstreamLinkState, DirectUpstreamLinkStatus, RobotBackend,
        direct_upstream_link_statuses,
    };

    #[test]
    fn direct_upstream_link_statuses_marks_exact_match_connected() {
        let upstream_endpoints = vec!["tcp/127.0.0.1:7447".to_string()];
        let link_destinations = vec!["tcp/127.0.0.1:7447".to_string()];

        assert_eq!(
            direct_upstream_link_statuses(&upstream_endpoints, &link_destinations),
            vec![DirectUpstreamLinkStatus {
                endpoint: "tcp/127.0.0.1:7447".to_string(),
                state: DirectUpstreamLinkState::Connected,
            }],
        );
    }

    #[test]
    fn direct_upstream_link_statuses_marks_literal_tcp_miss_disconnected() {
        let upstream_endpoints = vec!["tcp/192.168.1.20:7447".to_string()];
        let link_destinations = Vec::new();

        assert_eq!(
            direct_upstream_link_statuses(&upstream_endpoints, &link_destinations),
            vec![DirectUpstreamLinkStatus {
                endpoint: "tcp/192.168.1.20:7447".to_string(),
                state: DirectUpstreamLinkState::Disconnected,
            }],
        );
    }

    #[test]
    fn direct_upstream_link_statuses_marks_hostname_miss_unknown() {
        let upstream_endpoints = vec!["tcp/robot-a:7447".to_string()];
        let link_destinations = vec!["tcp/192.168.1.20:7447".to_string()];

        assert_eq!(
            direct_upstream_link_statuses(&upstream_endpoints, &link_destinations),
            vec![DirectUpstreamLinkStatus {
                endpoint: "tcp/robot-a:7447".to_string(),
                state: DirectUpstreamLinkState::Unknown,
            }],
        );
    }

    #[test]
    fn direct_upstream_link_statuses_marks_ipv6_literal_miss_disconnected() {
        let upstream_endpoints = vec!["tcp/[::1]:7447".to_string()];
        let link_destinations = Vec::new();

        assert_eq!(
            direct_upstream_link_statuses(&upstream_endpoints, &link_destinations),
            vec![DirectUpstreamLinkStatus {
                endpoint: "tcp/[::1]:7447".to_string(),
                state: DirectUpstreamLinkState::Disconnected,
            }],
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn backend_starts_with_empty_direct_upstream_links() {
        let backend = tokio::time::timeout(
            Duration::from_secs(5),
            RobotBackend::new(
                tokio::runtime::Handle::current(),
                Vec::new(),
                "/".to_string(),
            ),
        )
        .await
        .expect("backend startup should not block without direct upstream links")
        .expect("backend should build");

        let status = backend.status();

        assert!(status.local_router_endpoint.starts_with("tcp/127.0.0.1:"));
        assert_eq!(
            status,
            BackendStatus {
                local_router_endpoint: status.local_router_endpoint.clone(),
                direct_upstream_links: Vec::new(),
            }
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn backend_starts_healthy_when_upstream_router_is_unavailable() {
        let unavailable_endpoint = {
            let listener = TcpListener::bind("127.0.0.1:0")
                .expect("loopback listener should bind an ephemeral port");
            let endpoint = format!(
                "tcp/{}",
                listener
                    .local_addr()
                    .expect("loopback listener should have a local address")
            );
            drop(listener);
            endpoint
        };
        let upstream_endpoints = vec![unavailable_endpoint.clone()];

        let backend = tokio::time::timeout(
            Duration::from_secs(5),
            RobotBackend::new(
                tokio::runtime::Handle::current(),
                upstream_endpoints.clone(),
                "/".to_string(),
            ),
        )
        .await
        .expect("backend startup should not block on an unavailable upstream router")
        .expect("backend should build even when an upstream router is unavailable");
        let status = backend.status();

        assert!(status.local_router_endpoint.starts_with("tcp/127.0.0.1:"));
        assert_eq!(status.direct_upstream_links.len(), 1);
        let upstream_link: &DirectUpstreamLinkStatus = &status.direct_upstream_links[0];
        assert_eq!(upstream_link.endpoint, unavailable_endpoint);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn backend_returns_error_when_hidden_router_configuration_is_invalid() {
        let result = tokio::time::timeout(
            Duration::from_secs(5),
            RobotBackend::new(
                tokio::runtime::Handle::current(),
                vec!["not a Zenoh endpoint".to_string()],
                "/".to_string(),
            ),
        )
        .await
        .expect("backend startup should not time out");

        assert!(result.is_err());
    }
}
