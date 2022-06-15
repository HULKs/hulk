pub struct Hysteresis {
    last_evaluation: bool,
}

impl Hysteresis {
    pub fn new() -> Self {
        Self {
            last_evaluation: false,
        }
    }
    pub fn update_greater_than(&mut self, value: f32, threshold: f32, hysteresis: f32) -> bool {
        let evaluation = value
            > threshold
                + if self.last_evaluation {
                    -hysteresis
                } else {
                    hysteresis
                };
        self.last_evaluation = evaluation;
        evaluation
    }
}
