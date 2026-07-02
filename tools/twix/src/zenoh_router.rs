use std::{future::Future, net::TcpListener};

use color_eyre::{Result, eyre::Context as _};
use ros_z::config::RouterConfigBuilder;
use serde_json::json;
use zenoh::{Session, Wait};

const HIDDEN_ZENOH_ROUTER_START_ATTEMPTS: usize = 16;

pub struct HiddenZenohRouter {
    session: Session,
    local_endpoint: String,
}

impl HiddenZenohRouter {
    pub async fn start(upstream_endpoints: &[String]) -> Result<Self> {
        let (session, local_endpoint) = open_hidden_router_with(
            upstream_endpoints,
            allocate_loopback_endpoint,
            |config, _endpoint| async move {
                zenoh::open(config)
                    .await
                    .map_err(|error| color_eyre::eyre::eyre!(error))
                    .wrap_err("failed to open hidden Zenoh router")
            },
        )
        .await?;

        Ok(Self {
            session,
            local_endpoint,
        })
    }

    pub fn local_endpoint(&self) -> &str {
        &self.local_endpoint
    }

    pub fn session(&self) -> Session {
        self.session.clone()
    }
}

async fn open_hidden_router_with<AllocateEndpoint, Open, OpenFuture, Opened>(
    upstream_endpoints: &[String],
    mut allocate_endpoint: AllocateEndpoint,
    mut open: Open,
) -> Result<(Opened, String)>
where
    AllocateEndpoint: FnMut() -> Result<String, std::io::Error>,
    Open: FnMut(zenoh::Config, String) -> OpenFuture,
    OpenFuture: Future<Output = Result<Opened>>,
{
    for attempt in 1..=HIDDEN_ZENOH_ROUTER_START_ATTEMPTS {
        let local_endpoint = match allocate_endpoint() {
            Ok(endpoint) => endpoint,
            Err(error) if attempt < HIDDEN_ZENOH_ROUTER_START_ATTEMPTS => {
                log::debug!("failed to allocate hidden Zenoh router endpoint; retrying: {error:#}");
                continue;
            }
            Err(error) => {
                return Err(error).wrap_err("failed to allocate hidden Zenoh router endpoint");
            }
        };

        let config = build_hidden_router_config(&local_endpoint, upstream_endpoints)?;

        match open(config, local_endpoint.clone()).await {
            Ok(opened) => return Ok((opened, local_endpoint)),
            Err(error) if attempt < HIDDEN_ZENOH_ROUTER_START_ATTEMPTS => {
                log::debug!(
                    "failed to open hidden Zenoh router on {local_endpoint}; retrying with a new endpoint: {error:#}"
                );
            }
            Err(error) => {
                return Err(error).wrap_err_with(|| {
                    format!("failed to open hidden Zenoh router after {attempt} attempts")
                });
            }
        }
    }

    unreachable!("hidden Zenoh router start attempts loop must return")
}

fn build_hidden_router_config(
    local_endpoint: &str,
    upstream_endpoints: &[String],
) -> Result<zenoh::Config> {
    RouterConfigBuilder::new()
        .with_listen_endpoint(local_endpoint)
        .with_override(
            "connect/endpoints",
            json!(upstream_endpoints),
            "Twix hidden router upstream endpoints",
        )
        .with_override(
            "connect/timeout_ms",
            json!(0),
            "Do not block Twix startup on upstream endpoints",
        )
        .with_override(
            "connect/exit_on_failure",
            json!(false),
            "Keep Twix running when upstream endpoints are unavailable",
        )
        .build_config()
        .map_err(|error| color_eyre::eyre::eyre!(error))
        .wrap_err("failed to build hidden Zenoh router config")
}

impl Drop for HiddenZenohRouter {
    fn drop(&mut self) {
        if let Err(error) = self.session.close().wait() {
            log::error!("failed to close hidden Zenoh router: {error:#}");
        }
    }
}

fn allocate_loopback_endpoint() -> Result<String, std::io::Error> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(format!("tcp/127.0.0.1:{port}"))
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Error, ErrorKind},
        sync::{Arc, Mutex},
        time::Duration,
    };

    use super::{HiddenZenohRouter, open_hidden_router_with};
    use tokio::time::timeout;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn hidden_router_retries_with_a_new_endpoint_when_open_finds_the_port_claimed() {
        let allocated_endpoints = [
            "tcp/127.0.0.1:18001".to_string(),
            "tcp/127.0.0.1:18002".to_string(),
        ];
        let next_endpoint = Arc::new(Mutex::new(0));
        let attempted_endpoints = Arc::new(Mutex::new(Vec::new()));

        let (_, local_endpoint) = timeout(
            Duration::from_secs(5),
            open_hidden_router_with(
                &[],
                || {
                    let mut next_endpoint = next_endpoint.lock().unwrap();
                    let endpoint = allocated_endpoints[*next_endpoint].clone();
                    *next_endpoint += 1;
                    Ok(endpoint)
                },
                |_, endpoint| {
                    let attempted_endpoints = attempted_endpoints.clone();
                    async move {
                        attempted_endpoints.lock().unwrap().push(endpoint.clone());

                        if endpoint.ends_with(":18001") {
                            Err(Error::new(ErrorKind::AddrInUse, "address already in use").into())
                        } else {
                            Ok(())
                        }
                    }
                },
            ),
        )
        .await
        .expect("hidden router retry timed out")
        .expect("hidden router should retry with a fresh endpoint");

        assert_eq!(local_endpoint, "tcp/127.0.0.1:18002");
        assert_eq!(
            attempted_endpoints.lock().unwrap().as_slice(),
            ["tcp/127.0.0.1:18001", "tcp/127.0.0.1:18002"]
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn hidden_router_uses_private_loopback_endpoint() {
        let router = timeout(Duration::from_secs(5), HiddenZenohRouter::start(&[]))
            .await
            .expect("hidden router start timed out")
            .expect("hidden router should start");

        assert!(router.local_endpoint().starts_with("tcp/127.0.0.1:"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn hidden_routers_can_coexist() {
        let first = timeout(Duration::from_secs(5), HiddenZenohRouter::start(&[]))
            .await
            .expect("first hidden router start timed out")
            .expect("first hidden router should start");
        let second = timeout(Duration::from_secs(5), HiddenZenohRouter::start(&[]))
            .await
            .expect("second hidden router start timed out")
            .expect("second hidden router should start");

        assert_ne!(first.local_endpoint(), second.local_endpoint());
    }
}
