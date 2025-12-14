/// Measurement from an external time source not actively synchronized with the system clock.
use serde::{Deserialize, Serialize};

use crate::{builtin_interfaces::time::Time, std_msgs::header::Header};

#[repr(C)]
#[derive(Debug, Serialize, Deserialize)]
pub struct TimeReference {
    /// stamp is system time for which measurement was valid
    /// frame_id is not used
    pub header: Header,
    ///  corresponding time from this external source
    pub time_ref: Time,
    /// (optional) name of time source
    pub source: String,
}
