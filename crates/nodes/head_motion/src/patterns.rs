//! Stateful target sequences for scanning and glancing, without motor commands.

mod glance;
mod scan;

pub use glance::{GlanceState, GlanceTarget, GlanceTimeout};
pub use scan::{ScanKind, ScanOutput, ScanState, ScanTimeout, ScanWaypoint};
