use std::{time::Duration, vec};

use calibration::measurement::Measurement;
use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::Ground;
use framework::{MainOutput, PerceptionInput};
use itertools::Itertools;
use linear_algebra::{point, Point2};
use log::{info, warn};
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
    pub initial_stabilization_delay: Duration,
    pub current_measurements: Vec<Measurement>,
    pub current_primary_phase_is_calibration: bool,
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
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub calibration_state: MainOutput<CalibrationState>,
}

impl CalibrationController {
    pub fn new(_context: CreationContext) -> Result<Self> {
        info!("Calibration controller start");
        Ok(Self {
            current_calibration_state: CalibrationState::default(),
            look_at_list: generate_look_at_list().unwrap(),
            current_look_at_index: 0,
            look_at_dispatch_waiting: Duration::from_millis(500),
            initial_stabilization_delay: Duration::from_millis(2000),
            current_measurements: vec![],
            current_primary_phase_is_calibration: false,
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let calibration_grame_state_active = match context.primary_state {
            PrimaryState::Calibration => true,
            _ => false,
        };
        if !calibration_grame_state_active {
            self.current_calibration_state.phase = CalibrationPhase::INACTIVE;
            return Ok(MainOutputs::default());
        }

        let primary_state_transition_to_calibration =
            calibration_grame_state_active && !self.current_primary_phase_is_calibration;
        self.current_primary_phase_is_calibration = calibration_grame_state_active;

        let current_cycle_time = context.cycle_time;
        let current_calibration_phase = &self.current_calibration_state.phase;

        let changed_phase: Option<CalibrationPhase> = match current_calibration_phase {
            CalibrationPhase::INACTIVE => {
                if primary_state_transition_to_calibration {
                    info!(
                        "Calibration is activated, waiting for {}s until the Robot is stable.",
                        self.initial_stabilization_delay.as_secs_f32()
                    );

                    Some(CalibrationPhase::INITIALIZE {
                        started_time: *current_cycle_time,
                    })
                } else {
                    None
                }
            }
            CalibrationPhase::INITIALIZE {
                started_time: activated_time,
            } => match context.primary_state {
                PrimaryState::Calibration => {
                    let waiting_duration = current_cycle_time
                        .start_time
                        .duration_since(activated_time.start_time)
                        .unwrap_or(Duration::default());

                    if waiting_duration >= self.initial_stabilization_delay {
                        self.current_measurements = vec![];
                        Some(self.try_transition_to_look_at(*context.cycle_time))
                    } else {
                        None
                    }
                }
                _ => None,
            },
            CalibrationPhase::LOOKAT { dispatch_time, .. } => {
                let time_diff = current_cycle_time
                    .start_time
                    .duration_since(dispatch_time.start_time)
                    .unwrap_or(Duration::default());

                if time_diff > self.look_at_dispatch_waiting {
                    Some(CalibrationPhase::CAPTURE {
                        dispatch_time: *current_cycle_time,
                    })
                } else {
                    None
                }
            }
            CalibrationPhase::CAPTURE { dispatch_time } => {
                // TODO verify if this mess is indeed correct!
                let mut values_top = context
                    .measurement_top
                    .persistent
                    .iter()
                    .filter_map(|(time, measurement)| {
                        // TODO check if we really need this!
                        if *time >= dispatch_time.start_time {
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
                        // TODO check if we really need this!
                        if *time >= dispatch_time.start_time {
                            Some(measurement.iter().flatten())
                        } else {
                            None
                        }
                    })
                    .flatten()
                    .map(|m| (*m).clone())
                    .collect_vec();

                let outcome = if !values_bottom.is_empty() || !values_bottom.is_empty() {
                    // TODO Require both cameras to give values or another mechanism to track that.
                    self.current_measurements.append(&mut values_top);
                    self.current_measurements.append(&mut values_bottom);

                    Some(self.try_transition_to_look_at(*context.cycle_time))
                } else {
                    None
                };
                outcome
            }
            CalibrationPhase::PROCESS => {
                info!(
                    "Switching to process!, found {} measurements",
                    self.current_measurements.len()
                );
                // TODO Add processing logic
                warn!("Processing is not defined yet!");

                Some(CalibrationPhase::FINISH)
            }
            CalibrationPhase::FINISH => None,
        };

        if let Some(new_phase) = changed_phase {
            info!("Phase change detected: {:?}", new_phase);
            self.current_calibration_state.phase = new_phase;
        }

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
                info!("\tNothing else to LOOKAT, goto PROCESS");
                CalibrationPhase::PROCESS
            }
        }
    }

    fn get_next_look_at(&mut self) -> Option<(Point2<Ground>, Option<CameraPosition>)> {
        let current_index = self.current_look_at_index;
        self.current_look_at_index += 1;
        if current_index < self.look_at_list.len() {
            Some(self.look_at_list[current_index])
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
