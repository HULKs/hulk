/// Detects an falling edge of two state sensor reading
#[derive(Default)]
pub struct TapDetector {
    last_reading: bool,
    is_single_tapped: bool,
}

impl TapDetector {
    pub fn update(&mut self, sensor_reading: bool) {
        self.is_single_tapped = self.last_reading && !sensor_reading;
        self.last_reading = sensor_reading;
    }

    pub fn is_single_tapped(&self) -> bool {
        self.is_single_tapped
    }
}
