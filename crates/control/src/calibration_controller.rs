use std::{
    time::{Duration, SystemTime},
    vec,
};

use calibration::measurement::Measurement;
use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::Ground;
use framework::{deserialize_not_implemented, MainOutput, PerceptionInput};
use itertools::Itertools;
use linear_algebra::{point, Point2};
use log::{log, Level};
use serde::{Deserialize, Serialize};
use types::{
    camera_position::CameraPosition,
    cycle_time::CycleTime,
    primary_state::PrimaryState,
    world_state::{CalibrationPhase, CalibrationState},
};

#[derive(Deserialize, Serialize)]
pub struct CalibrationController {
    pub current_calibration_state: CalibrationState,
    pub look_at_list: Vec<(Point2<Ground>, Option<CameraPosition>)>,
    pub current_look_at_index: usize,
    pub look_at_dispatch_waiting: Duration,
    #[serde(skip, default = "deserialize_not_implemented")]
    pub current_measurements: Vec<Measurement>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    primary_state: Input<PrimaryState, "primary_state">,
    cycle_time: Input<CycleTime, "cycle_time">,
    measurement_bottom:
        PerceptionInput<Option<Measurement>, "VisionBottom", "calibration_measurement?">,
    measurement_top: PerceptionInput<Option<Measurement>, "VisionTop", "calibration_measurement?">,
    // measurement_bottom:
    //     PerceptionInput<Option<SystemTime>, "VisionBottom", "calibration_measurement?">,
    // measurement_top: PerceptionInput<Option<SystemTime>, "VisionTop", "calibration_measurement?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub calibration_state: MainOutput<CalibrationState>,
}

impl CalibrationController {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            current_calibration_state: CalibrationState::default(),
            look_at_list: generate_look_at_list().unwrap(),
            current_look_at_index: 0,
            look_at_dispatch_waiting: Duration::from_millis(500),
            current_measurements: vec![],
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let continue_processing =
            match (context.primary_state, &self.current_calibration_state.phase) {
                (
                    PrimaryState::Calibration,
                    CalibrationPhase::LOOKAT { .. }
                    | CalibrationPhase::CAPTURE { .. }
                    | CalibrationPhase::PROCESS
                    | CalibrationPhase::FINISH,
                ) => true,
                (_, _) => false,
            };
        if !continue_processing {
            return Ok(MainOutputs::default());
        }

        let current_phase = &self.current_calibration_state.phase;
        let current_cycle_time = context.cycle_time;

        let new_phase = match current_phase {
            CalibrationPhase::INACTIVE => match context.primary_state {
                PrimaryState::Calibration => {
                    log!(Level::Info, "Calibration is activated! Go-to LOOKAT");
                    self.current_measurements = vec![];
                    self.try_transition_to_look_at(*context.cycle_time)
                }
                _ => CalibrationPhase::INACTIVE,
            },
            CalibrationPhase::LOOKAT { dispatch_time, .. } => {
                let time_diff = current_cycle_time
                    .start_time
                    .duration_since(dispatch_time.start_time)
                    .unwrap_or(Duration::default());

                if time_diff > self.look_at_dispatch_waiting {
                    log!(Level::Info, "Look-at reached. Goto CAPTURE");
                    CalibrationPhase::CAPTURE {
                        dispatch_time: *current_cycle_time,
                    }
                } else {
                    current_phase.clone()
                }
            }
            CalibrationPhase::CAPTURE { dispatch_time } => {
                // TODO verify if this mess is indeed correct!
                let mut values_top = context
                    .measurement_top
                    .persistent
                    .iter()
                    .filter_map(|(time, measurement)| {
                        if *time > dispatch_time.start_time {
                            Some(measurement.iter().flatten())
                        } else {
                            None
                        }
                    })
                    .flatten()
                    .map(|m| (*m).clone())
                    .collect_vec();

                let mut values_bottom = context
                    .measurement_bottom
                    .persistent
                    .iter()
                    .filter_map(|(time, measurement)| {
                        if *time > dispatch_time.start_time {
                            Some(measurement.iter().flatten())
                        } else {
                            None
                        }
                    })
                    .flatten()
                    .map(|m| (*m).clone())
                    .collect_vec();

                let outcome = if !values_bottom.is_empty() || !values_bottom.is_empty() {
                    // TODO complete this later
                    // self.current_measurements.append(&mut values_top);
                    // self.current_measurements.append(&mut values_bottom);

                    log!(Level::Info, "\tFound captures, try goto next LOOKAT");
                    // Once this capture is done, goto the next look-at
                    self.try_transition_to_look_at(*context.cycle_time)
                } else {
                    current_phase.clone()
                };
                outcome
            }
            CalibrationPhase::PROCESS => {
                log!(Level::Info, "Switching to process!");
                /// Proces...
                log!(Level::Info, "Transitioning to finished!");
                CalibrationPhase::FINISH
            }
            CalibrationPhase::FINISH => CalibrationPhase::FINISH,
        };

        Ok(MainOutputs {
            calibration_state: self.current_calibration_state.clone().into(),
        })
    }

    fn try_transition_to_look_at(&mut self, dispatch_time: CycleTime) -> CalibrationPhase {
        let next_look_at = self.get_next_look_at();
        match next_look_at {
            Some((target, camera)) => CalibrationPhase::LOOKAT {
                camera,
                target,
                dispatch_time,
            },
            None => {
                log!(Level::Info, "\tNothing else to LOOKAT, goto PROCESS");
                CalibrationPhase::PROCESS
            }
        }
    }

    fn get_next_look_at(&mut self) -> Option<(Point2<Ground>, Option<CameraPosition>)> {
        let next_index = self.current_look_at_index + 1;
        if next_index < self.look_at_list.len() {
            Some(self.look_at_list[next_index])
        } else {
            None
        }
    }
}

// x_min:f32, x_max:f32, x_steps:usize, y_min:f32, y_max:f32,y_steps:usize
fn generate_look_at_list() -> Result<Vec<(Point2<Ground>, Option<CameraPosition>)>> {
    let look_at_points: Vec<Point2<Ground>> = vec![
        point!(1.0, 0.0),
        point!(1.0, -0.5),
        point!(3.0, -0.5),
        point!(3.0, 0.0),
        point!(3.0, 0.5),
        point!(1.0, -0.5),
    ];

    let attach_camera_to_lookat = |point: &Point2<Ground>,
                                   camera_position: &CameraPosition|
     -> (Point2<Ground>, Option<CameraPosition>) {
        (point.clone(), Some(*camera_position))
    };

    Ok(look_at_points
        .iter()
        .map(|&point| attach_camera_to_lookat(&point, &CameraPosition::Top))
        .collect_vec())
}
