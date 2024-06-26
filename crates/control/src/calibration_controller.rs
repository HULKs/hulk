use std::{time::Duration, vec};

use calibration::{
    center_circle::{measurement::Measurement, residuals::Residuals},
    corrections::Corrections,
    solve,
};
use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::Ground;
use framework::{AdditionalOutput, MainOutput, PerceptionInput};
use itertools::Itertools;
use linear_algebra::{point, Point2};
use log::info;
use serde::{Deserialize, Serialize};
use types::{
    calibration::{CalibrationCaptureResponse, CalibrationCommand},
    camera_position::CameraPosition,
    cycle_time::CycleTime,
    field_dimensions::FieldDimensions,
    primary_state::PrimaryState,
};

#[derive(Deserialize, Serialize)]
pub struct CalibrationController {
    pub current_calibration_command: CalibrationCommand,
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
    measurement_bottom: PerceptionInput<
        CalibrationCaptureResponse<Measurement>,
        "VisionBottom",
        "calibration_measurement",
    >,
    measurement_top: PerceptionInput<
        CalibrationCaptureResponse<Measurement>,
        "VisionTop",
        "calibration_measurement",
    >,
    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    calibration_measurements: AdditionalOutput<Vec<Measurement>, "calibration_measurements">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub calibration_command: MainOutput<CalibrationCommand>,
}

impl CalibrationController {
    pub fn new(_context: CreationContext) -> Result<Self> {
        info!("Calibration controller start");
        Ok(Self {
            current_calibration_command: CalibrationCommand::default(),
            look_at_list: generate_look_at_list().unwrap(),
            current_look_at_index: 0,
            look_at_dispatch_waiting: Duration::from_millis(500),
            initial_stabilization_delay: Duration::from_millis(2000),
            current_measurements: vec![],
            current_primary_phase_is_calibration: false,
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let calibration_grame_state_active =
            matches!(context.primary_state, PrimaryState::Calibration);
        if !calibration_grame_state_active {
            self.current_calibration_command = CalibrationCommand::INACTIVE;
            return Ok(MainOutputs::default());
        }

        let primary_state_transitioned_to_calibration =
            calibration_grame_state_active && !self.current_primary_phase_is_calibration;
        self.current_primary_phase_is_calibration = calibration_grame_state_active;

        if primary_state_transitioned_to_calibration {
            self.current_measurements = vec![];
        }

        let current_cycle_time = context.cycle_time;
        let changed_command: Option<CalibrationCommand> = self.get_next_command(
            primary_state_transitioned_to_calibration,
            current_cycle_time,
            &context,
        );

        if let Some(new_phase) = changed_command {
            info!("Phase change detected: {:?}", new_phase);
            self.current_calibration_command = new_phase;
        }

        context
            .calibration_measurements
            .fill_if_subscribed(|| self.current_measurements.clone());
        Ok(MainOutputs {
            calibration_command: self.current_calibration_command.clone().into(),
        })
    }

    fn get_next_command(
        &mut self,
        primary_state_transition_to_calibration: bool,
        current_cycle_time: &CycleTime,
        context: &CycleContext,
    ) -> Option<CalibrationCommand> {
        match self.current_calibration_command {
            CalibrationCommand::INACTIVE => {
                if primary_state_transition_to_calibration {
                    info!(
                        "Calibration is activated, waiting for {}s until the Robot is stable.",
                        self.initial_stabilization_delay.as_secs_f32()
                    );

                    Some(CalibrationCommand::INITIALIZE {
                        started_time: *current_cycle_time,
                    })
                } else {
                    None
                }
            }
            CalibrationCommand::INITIALIZE {
                started_time: activated_time,
            } => match context.primary_state {
                PrimaryState::Calibration => {
                    let waiting_duration = current_cycle_time
                        .start_time
                        .duration_since(activated_time.start_time)
                        .unwrap_or_default();

                    if waiting_duration >= self.initial_stabilization_delay {
                        Some(self.try_transition_to_look_at(*context.cycle_time))
                    } else {
                        None
                    }
                }
                _ => None,
            },
            CalibrationCommand::LOOKAT { dispatch_time, .. } => {
                let time_diff = current_cycle_time
                    .start_time
                    .duration_since(dispatch_time.start_time)
                    .unwrap_or_default();

                if time_diff > self.look_at_dispatch_waiting {
                    Some(CalibrationCommand::CAPTURE {
                        dispatch_time: *current_cycle_time,
                    })
                } else {
                    None
                }
            }
            CalibrationCommand::CAPTURE { dispatch_time } => {
                // TODO decide a better way to handle the two cameras. Perhaps set a capture position like in the look-at command?
                let (values_top, goto_next_position_top) =
                    collect_filtered_values(&context.measurement_top, &dispatch_time);
                let (values_bottom, goto_next_position_bottom) =
                    collect_filtered_values(&context.measurement_bottom, &dispatch_time);

                if let Some(measurement) = values_top {
                    self.current_measurements.push(measurement);
                }
                if let Some(measurement) = values_bottom {
                    self.current_measurements.push(measurement);
                }

                if goto_next_position_top || goto_next_position_bottom {
                    Some(self.try_transition_to_look_at(*context.cycle_time))
                } else {
                    None
                }
            }
            CalibrationCommand::PROCESS => {
                info!(
                    "Switching to process!, found {} measurements",
                    self.current_measurements.len()
                );

                let solved_result = solve::<Measurement, Residuals>(
                    Corrections::default(),
                    self.current_measurements.clone(),
                    context.field_dimensions.clone(),
                );

                info!("Calibration complete! Corrections: {solved_result:?}");
                Some(CalibrationCommand::FINISH)
            }
            CalibrationCommand::FINISH => None,
        }
    }

    fn try_transition_to_look_at(&mut self, dispatch_time: CycleTime) -> CalibrationCommand {
        let next_look_at = self.get_next_look_at();

        match next_look_at {
            Some((target, camera)) => CalibrationCommand::LOOKAT {
                camera,
                target,
                dispatch_time,
            },
            None => {
                info!("\tNothing else to LOOKAT, goto PROCESS");
                CalibrationCommand::PROCESS
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

fn collect_filtered_values(
    measurement_perception_input: &PerceptionInput<Vec<&CalibrationCaptureResponse<Measurement>>>,
    original_dispatch_time: &CycleTime,
) -> (Option<Measurement>, bool) {
    let result = measurement_perception_input
        .persistent
        .iter()
        .flat_map(|(_cycle_timestamp, measurements)| measurements.iter())
        .find(|&measurement| match measurement {
            CalibrationCaptureResponse::CommandRecieved {
                dispatch_time,
                output: _,
            } => original_dispatch_time.start_time == dispatch_time.start_time,
            CalibrationCaptureResponse::Idling => false,
            CalibrationCaptureResponse::RetriesExceeded { dispatch_time } => {
                original_dispatch_time.start_time == dispatch_time.start_time
            }
        });

    let mut retries_exceeded = false;
    let mut value_found = false;
    let measurement: Option<Measurement> = result.and_then(|value| match value {
        CalibrationCaptureResponse::Idling => None,
        CalibrationCaptureResponse::CommandRecieved {
            dispatch_time: _,
            output,
        } => {
            if output.is_some() {
                value_found = true;
                output.clone()
            } else {
                None
            }
        }
        CalibrationCaptureResponse::RetriesExceeded { dispatch_time: _ } => {
            retries_exceeded = true;
            None
        }
    });

    (measurement, retries_exceeded || value_found)
}

// TODO Add fancier logic to either set this via parameters OR detect the location, walk, etc
fn generate_look_at_list() -> Result<Vec<(Point2<Ground>, Option<CameraPosition>)>> {
    let look_at_points: Vec<Point2<Ground>> = vec![
        point!(1.0, 0.0),
        point!(1.0, -0.5),
        point!(3.0, -0.5),
        point!(3.0, 0.0),
        point!(3.0, 0.5),
        point!(1.0, -0.5),
    ];

    let attach_camera_to_lookat =
        |point: &Point2<Ground>,
         camera_position: &CameraPosition|
         -> (Point2<Ground>, Option<CameraPosition>) { (*point, Some(*camera_position)) };

    Ok(look_at_points
        .iter()
        .map(|&point| attach_camera_to_lookat(&point, &CameraPosition::Top))
        .collect_vec())
}
