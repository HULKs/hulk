use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Default, Deserialize, Serialize)]
/// Detects an falling edge of two state sensor reading
pub struct TapDetector {
    last_reading: bool,
    pub is_single_tapped: bool,
    pub is_double_tapped: bool,
    #[serde(skip_serializing, skip_deserializing)] //this is suppose to somehow avoid the error
    last_tap_time: Option<Instant>, //stores the time value for the last tapped
}

impl TapDetector {
    pub fn update(&mut self, sensor_reading: bool) {
        self.is_single_tapped = self.last_reading && !sensor_reading;
        //self.last_reading = sensor_reading;

        self.is_double_tapped = false;
        if self.is_single_tapped {
            if let Some(last_tap_time) = self.last_tap_time {
                //defining the last_tap_time
                let time_since_last_tap: Duration = Instant::now().duration_since(last_tap_time); //defining the Duration
                if time_since_last_tap <= Duration::from_millis(500) {
                    //compares the type Duration with the 1000 ms, if it's less than that, double tap is true
                    self.is_single_tapped = false;
                    self.is_double_tapped = true;
                } 
            }
        } 
        self.last_reading = sensor_reading;
    }

  
}
// to detect double tap, detect two falling edges,
// we need detect tap time, but the time between tap has to not be too long, else it should be a single tap,
// store the time between taps, if falling edge detected within 1s, then double tap detected.
// go into button filter then and add double tap there as well
// then this double tap can be the entry to animation mode?
// add a pb fn is_double_tapped?
