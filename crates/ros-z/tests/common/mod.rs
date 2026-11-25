use ros_z::context::{Context, ContextBuilder};
use serde_json::json;

pub struct TestRouter {
    endpoint: String,
    _session: zenoh::Session,
}

impl TestRouter {
    pub async fn new() -> Self {
        let config = zenoh::Config::from_deserializer(json!({
            "mode": "router",
            "listen": { "endpoints": ["tcp/127.0.0.1:0"] },
            "scouting": { "multicast": { "enabled": false }, "gossip": { "enabled": false } },
        }))
        .expect("test router config");
        let session = zenoh::open(config).await.expect("open test router");
        let endpoint = session
            .info()
            .locators()
            .await
            .into_iter()
            .next()
            .expect("bound test router endpoint")
            .to_string();

        Self {
            endpoint,
            _session: session,
        }
    }
}

pub async fn create_context_with_router(router: &TestRouter) -> ros_z::Result<Context> {
    // With scouting disabled, clients rely on the router instead of needing direct peer links.
    ContextBuilder::isolated_for_test()
        .with_mode("client")
        .with_listen_endpoints([] as [&str; 0])
        .with_connect_endpoints([router.endpoint.as_str()])
        .build()
        .await
}
