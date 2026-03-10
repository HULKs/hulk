pub mod binding;
pub mod commands;
pub mod events;

use std::time::{Duration, Instant};

pub use crate::config::ViewerConfig;
pub use crate::discovery_types::{DiscoveredParameter, DiscoveredPublisher, DiscoveredSession};
pub use binding::{ParameterReference, SourceBindingInfo, SourceBindingRequest, StreamId};
pub use commands::WorkerCommand;
pub use events::{
    DiscoveryOp, DisplayedRecord, RecordChunkSource, WorkerEvent, WorkerEventEnvelope,
    WorkerWakeNotifier,
};

pub fn should_emit_scrub_command(last_emitted: Instant, now: Instant, debounce: Duration) -> bool {
    now.saturating_duration_since(last_emitted) >= debounce
}

#[cfg(test)]
mod tests {
    use super::should_emit_scrub_command;
    use std::time::{Duration, Instant};

    #[test]
    fn scrub_debounce_blocks_rapid_updates() {
        let now = Instant::now();
        let debounce = Duration::from_millis(200);

        assert!(!should_emit_scrub_command(
            now,
            now + Duration::from_millis(100),
            debounce
        ));
        assert!(should_emit_scrub_command(
            now,
            now + Duration::from_millis(220),
            debounce
        ));
    }
}
