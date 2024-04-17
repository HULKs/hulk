pub struct RecordingTrigger {
    recording_interval: usize,
    counter: usize,
}

impl RecordingTrigger {
    pub fn new(recording_interval: usize) -> Self {
        Self {
            recording_interval,
            counter: 0,
        }
    }

    pub fn update(&mut self) {
        self.counter = (self.counter + 1) % self.recording_interval;
    }

    pub fn should_record(&self) -> bool {
        self.recording_interval != 0 && self.counter == 0
    }
}
