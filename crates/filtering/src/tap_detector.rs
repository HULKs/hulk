/// Detects an falling edge of two state sensor reading
pub struct TapDetector {
    last_reading: bool,
    is_single_tapped: bool,
}

impl TapDetector {
    pub fn new() -> Self {
        Self {
            last_reading: false,
            is_single_tapped: false,
        }
    }

    pub fn update(&mut self, sensor_reading: bool) {
        if self.last_reading && !sensor_reading {
            self.is_single_tapped = true;
        } else {
            self.is_single_tapped = false;
        }
        self.last_reading = sensor_reading;
    }

    pub fn is_single_tapped(&self) -> bool {
        self.is_single_tapped
    }
}
