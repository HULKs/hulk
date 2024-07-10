use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct DebounceButton {
    last_button_touched: bool,
    button_touched_time: SystemTime,
}

impl Default for DebounceButton {
    fn default() -> Self {
        Self {
            last_button_touched: Default::default(),
            button_touched_time: UNIX_EPOCH,
        }
    }
}

impl DebounceButton {
    pub fn debounce_button(
        &mut self,
        button_touched: bool,
        current_time: SystemTime,
        timeout: Duration,
    ) -> bool {
        let button_touched_initially = button_touched && !self.last_button_touched;
        if button_touched_initially {
            self.button_touched_time = current_time;
        }
        self.last_button_touched = button_touched;

        button_touched
            && current_time
                .duration_since(self.button_touched_time)
                .unwrap()
                >= timeout
    }
}
