// # Measurement from an external time source not actively synchronized with the system clock.

use serde::{Deserialize, Serialize};

use crate::{builtin_interfaces::time::Time, std_msgs::header::Header};

#[derive(Debug, Serialize, Deserialize)]
pub struct TimeReference {
    pub header: Header, // # stamp is system time for which measurement was valid
    // # frame_id is not used
    pub time_ref: Time, // # corresponding time from this external source
    pub source: String, // # (optional) name of time source
}
