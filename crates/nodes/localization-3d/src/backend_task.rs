use std::future::ready;

use color_eyre::{Result, eyre::Context as _};
use localization_factrs::VinsBackend;
use ros_z::{pubsub::Publisher, time::Time};
use tokio::task::JoinHandle;
use types::time_wrapper::TimeWrapper;

use crate::diagnostics::SolveDiagnostics;

pub(crate) fn spawn_backend_task(
    mut backend: VinsBackend,
    solve_diagnostics_publisher: Publisher<TimeWrapper<SolveDiagnostics>>,
) -> JoinHandle<Result<()>> {
    let runtime = tokio::runtime::Handle::current();
    tokio::task::spawn_blocking(move || -> Result<()> {
        loop {
            let Some(result) = backend.solve_next_blocking()? else {
                continue;
            };

            runtime
                .block_on(solve_diagnostics_publisher.publish_if_subscribed(|| {
                    let diagnostics = backend
                        .compute_last_solve_diagnostics()
                        .expect("diagnostics are available after a successful solve");

                    ready(TimeWrapper {
                        time: Time::from_wallclock(result.time),
                        inner: diagnostics.into(),
                    })
                }))
                .wrap_err("failed to publish solve diagnostics")?;
        }
    })
}
