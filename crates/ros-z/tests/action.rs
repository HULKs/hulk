// Action-related integration tests
// This module organizes all action-related tests into a single integration test

mod action {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use ros_z::context::ContextBuilder;

    static TEST_ID: AtomicUsize = AtomicUsize::new(0);

    fn unique_test_token(scope: &str) -> String {
        let id = TEST_ID.fetch_add(1, Ordering::Relaxed);
        let scope = scope
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() || ch == '_' {
                    ch
                } else {
                    '_'
                }
            })
            .collect::<String>();
        format!("{}_pid_{}_{}", scope, std::process::id(), id)
    }

    fn unique_namespace(scope: &str) -> String {
        format!("/{}", unique_test_token(scope))
    }

    fn unique_action_name(scope: &str) -> String {
        format!("/{}", unique_test_token(scope))
    }

    fn isolated_context(_scope: &str) -> ContextBuilder {
        ContextBuilder::default()
            .disable_multicast_scouting()
            .with_connect_endpoints(std::iter::empty::<&str>())
            .with_listen_endpoints(["tcp/localhost:0"])
    }

    #[cfg(test)]
    mod isolation_tests {
        use super::isolated_context;
        #[tokio::test(flavor = "multi_thread")]
        async fn isolated_context_builds_without_ambient_router() -> zenoh::Result<()> {
            let _ctx = isolated_context("isolated_context_builds_without_ambient_router")
                .build()
                .await?;
            Ok(())
        }
    }

    mod client;
    mod communication;
    mod expiration;
    mod goal_handle;
    mod goal_state_machine;
    mod graph;
    mod interaction;
    mod remapping;
    mod server;
    mod wait;
}
