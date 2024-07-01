use std::{time::Duration, vec};

use calibration::{corrections::Corrections, measurement::Measurement, solve};
use color_eyre::Result;
use context_attribute::context;
use coordinate_systems::Ground;
use framework::{AdditionalOutput, MainOutput, PerceptionInput};
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
    pub current_primary_phase_is_calibration: bool,
    pub current_look_at_index: usize,
    pub current_calibration_command: CalibrationCommand,
    pub current_measurements: Vec<Measurement>,
    pub last_calibration_corrections: Option<Corrections>,
    pub look_at_list: Vec<(Point2<Ground>, CameraPosition)>,
    pub last_capture_retries: u32,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    primary_state: Input<PrimaryState, "primary_state">,
    cycle_time: Input<CycleTime, "cycle_time">,
    measurement_bottom: PerceptionInput<
        Option<CalibrationCaptureResponse<Measurement>>,
        "VisionBottom",
        "calibration_measurement?",
    >,
    measurement_top: PerceptionInput<
        Option<CalibrationCaptureResponse<Measurement>>,
        "VisionTop",
        "calibration_measurement?",
    >,
    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    look_at_dispatch_wait_duration:
        Parameter<Duration, "calibration_controller.look_at_dispatch_wait_duration">,
    initial_to_calibration_stabilization_delay:
        Parameter<Duration, "calibration_controller.initial_to_calibration_stabilization_delay">,
    max_retries_per_capture: Parameter<u32, "calibration_controller.max_retries_per_capture">,

    calibration_measurements: AdditionalOutput<Vec<Measurement>, "calibration_measurements">,
    last_calibration_corrections:
        AdditionalOutput<Option<Corrections>, "last_calibration_corrections">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub calibration_command: MainOutput<CalibrationCommand>,
}

impl CalibrationController {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            current_calibration_command: CalibrationCommand::default(),
            look_at_list: generate_look_at_list(),
            current_look_at_index: 0,
            current_measurements: vec![],
            current_primary_phase_is_calibration: false,
            last_calibration_corrections: None,
            last_capture_retries: 0,
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let calibration_grame_state_active =
            matches!(context.primary_state, PrimaryState::Calibration);
        if !calibration_grame_state_active {
            self.current_calibration_command = CalibrationCommand::Inactive;
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
        context
            .last_calibration_corrections
            .fill_if_subscribed(|| self.last_calibration_corrections);
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
        let look_at_dispatch_waiting = *context.look_at_dispatch_wait_duration;
        let initial_stabilization_delay = *context.initial_to_calibration_stabilization_delay;
        match self.current_calibration_command {
            CalibrationCommand::Inactive => {
                if primary_state_transition_to_calibration {
                    info!(
                        "Calibration is activated, waiting for {}s until the Robot is stable.",
                        initial_stabilization_delay.as_secs_f32()
                    );

                    Some(CalibrationCommand::Initialize {
                        started_time: *current_cycle_time,
                    })
                } else {
                    None
                }
            }
            CalibrationCommand::Initialize {
                started_time: activated_time,
            } => match context.primary_state {
                PrimaryState::Calibration => {
                    let waiting_duration = current_cycle_time
                        .start_time
                        .duration_since(activated_time.start_time)
                        .unwrap_or_default();

                    if waiting_duration >= initial_stabilization_delay {
                        Some(self.try_transition_to_look_at(*context.cycle_time))
                    } else {
                        None
                    }
                }
                _ => None,
            },
            CalibrationCommand::LookAt {
                dispatch_time,
                camera,
                ..
            } => {
                let time_diff = current_cycle_time
                    .start_time
                    .duration_since(dispatch_time.start_time)
                    .unwrap_or_default();

                if time_diff > look_at_dispatch_waiting {
                    self.last_capture_retries = 0;
                    Some(CalibrationCommand::Capture {
                        camera,
                        dispatch_time: *current_cycle_time,
                    })
                } else {
                    None
                }
            }
            CalibrationCommand::Capture {
                dispatch_time,
                camera,
            } => {
                let calibration_response = match camera {
                    CameraPosition::Top => {
                        collect_filtered_values(&context.measurement_top, &dispatch_time)
                    }
                    CameraPosition::Bottom => {
                        collect_filtered_values(&context.measurement_bottom, &dispatch_time)
                    }
                };

                calibration_response.and_then(|response| {
                    let goto_next_lookat = if let Some(measurement) = response.measurement {
                        self.current_measurements.push(measurement);
                        true
                    } else {
                        self.last_capture_retries += 1;
                        self.last_capture_retries > *context.max_retries_per_capture
                    };
                    if goto_next_lookat {
                        Some(self.try_transition_to_look_at(*context.cycle_time))
                    } else {
                        None
                    }
                })
            }
            CalibrationCommand::Process => {
                info!(
                    "Switching to process!, found {} measurements",
                    self.current_measurements.len()
                );

                let solved_result = solve(
                    Corrections::default(),
                    self.current_measurements.clone(),
                    *context.field_dimensions,
                );

                info!("Calibration complete! Corrections: {solved_result:?}");
                self.last_calibration_corrections = Some(solved_result);
                Some(CalibrationCommand::Finish)
            }
            CalibrationCommand::Finish => None,
        }
    }

    fn try_transition_to_look_at(&mut self, dispatch_time: CycleTime) -> CalibrationCommand {
        match self.get_next_look_at() {
            Some((target, camera)) => CalibrationCommand::LookAt {
                camera,
                target,
                dispatch_time,
            },
            None => {
                info!("\tNothing else to LOOKAT, goto PROCESS");
                CalibrationCommand::Process
            }
        }
    }

    fn get_next_look_at(&mut self) -> Option<(Point2<Ground>, CameraPosition)> {
        let current_index = self.current_look_at_index;
        self.current_look_at_index += 1;
        self.look_at_list.get(current_index).copied()
    }
}

fn collect_filtered_values(
    measurement_perception_input: &PerceptionInput<
        Vec<Option<&CalibrationCaptureResponse<Measurement>>>,
    >,
    original_dispatch_time: &CycleTime,
) -> Option<CalibrationCaptureResponse<Measurement>> {
    measurement_perception_input
        .persistent
        .iter()
        .flat_map(|(_cycle_timestamp, measurements)| measurements.iter())
        .flatten()
        .find(|&measurement| {
            let CalibrationCaptureResponse {
                dispatch_time,
                measurement: _,
            } = measurement;
            original_dispatch_time.start_time == dispatch_time.start_time
        })
        .cloned()
        .cloned()
}

// TODO Add fancier logic to either set this via parameters OR detect the location, walk, etc
fn generate_look_at_list() -> Vec<(Point2<Ground>, CameraPosition)> {
    let look_at_points: Vec<Point2<Ground>> = vec![
        point!(1.0, 0.0),
        point!(1.0, -0.5),
        point!(3.0, -0.5),
        point!(3.0, 0.0),
        point!(3.0, 0.5),
        point!(1.0, -0.5),
    ];

    look_at_points
        .iter()
        .map(|&point| (point, CameraPosition::Top))
        .collect()
}
