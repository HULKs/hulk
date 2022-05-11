use std::time::SystemTime;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum FilteredGameState {
    Ready { changed_time: SystemTime },
    Initial,
    Set,
    Playing { changed_time: SystemTime },
    Finished,
}
