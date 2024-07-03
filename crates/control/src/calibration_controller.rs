use std::{time::Duration, vec};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use calibration::{corrections::Corrections, measurement::Measurement, solve};
use context_attribute::context;
use coordinate_systems::Ground;
use framework::{AdditionalOutput, MainOutput, PerceptionInput};
use linear_algebra::{point, Point2};
use types::{
    calibration::{CalibrationCaptureResponse, CalibrationCommand},
    camera_position::CameraPosition,
    cycle_time::CycleTime,
    field_dimensions::FieldDimensions,
    primary_state::PrimaryState,
};

#[derive(Deserialize, Serialize)]
pub struct CalibrationController {
    pub current_look_at_index: usize,
    pub current_calibration_state: CalibrationState,
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
    pub calibration_command: MainOutput<Option<CalibrationCommand>>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
enum CalibrationState {
    #[default]
    Inactive,
    Initialize {
        started_time: CycleTime,
    },
    LookAt {
        target: Point2<Ground>,
        camera: CameraPosition,
        dispatch_time: CycleTime,
    },
    Capture {
        camera: CameraPosition,
        dispatch_time: CycleTime,
    },
    Process,
    Finish,
}

impl From<CalibrationState> for Option<CalibrationCommand> {
    fn from(value: CalibrationState) -> Self {
        match value {
            CalibrationState::LookAt {
                target,
                camera,
                dispatch_time,
            } => Some(CalibrationCommand::LookAt {
                target,
                camera,
                dispatch_time,
            }),
            CalibrationState::Capture {
                camera,
                dispatch_time,
            } => Some(CalibrationCommand::Capture {
                camera,
                dispatch_time,
            }),
            _ => None,
        }
    }
}

impl CalibrationController {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            current_calibration_state: CalibrationState::default(),
            look_at_list: generate_look_at_list(),
            current_look_at_index: 0,
            current_measurements: vec![],
            last_calibration_corrections: None,
            last_capture_retries: 0,
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        if *context.primary_state != PrimaryState::Calibration {
            self.current_calibration_state = CalibrationState::Inactive;
            return Ok(MainOutputs::default());
        }

        if let Some(new_state) = self.get_next_state(&context) {
            self.current_calibration_state = new_state;
        }

        context
            .calibration_measurements
            .fill_if_subscribed(|| self.current_measurements.clone());
        context
            .last_calibration_corrections
            .fill_if_subscribed(|| self.last_calibration_corrections);

        let command: Option<CalibrationCommand> = self.current_calibration_state.clone().into();
        Ok(MainOutputs {
            calibration_command: command.into(),
        })
    }

    fn get_next_state(&mut self, context: &CycleContext) -> Option<CalibrationState> {
        let look_at_dispatch_waiting = *context.look_at_dispatch_wait_duration;
        let initial_stabilization_delay = *context.initial_to_calibration_stabilization_delay;
        let current_cycle_time = context.cycle_time;
        match self.current_calibration_state {
            CalibrationState::Inactive => {
                self.current_measurements = vec![];
                Some(CalibrationState::Initialize {
                    started_time: *current_cycle_time,
                })
            }
            CalibrationState::Initialize {
                started_time: activated_time,
            } => {
                let waiting_duration = current_cycle_time
                    .start_time
                    .duration_since(activated_time.start_time)
                    .unwrap_or_default();

                if waiting_duration >= initial_stabilization_delay {
                    Some(self.get_next_look_at_or_processing(*context.cycle_time))
                } else {
                    None
                }
            }
            CalibrationState::LookAt {
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
                    Some(CalibrationState::Capture {
                        camera,
                        dispatch_time: *current_cycle_time,
                    })
                } else {
                    None
                }
            }
            CalibrationState::Capture {
                dispatch_time,
                camera,
            } => {
                let calibration_response = collect_filtered_values(
                    match camera {
                        CameraPosition::Top => &context.measurement_top,
                        CameraPosition::Bottom => &context.measurement_bottom,
                    },
                    &dispatch_time,
                );

                calibration_response.and_then(|response| {
                    let goto_next_lookat = if let Some(measurement) = response.measurement {
                        self.current_measurements.push(measurement);
                        true
                    } else {
                        self.last_capture_retries += 1;
                        self.last_capture_retries > *context.max_retries_per_capture
                    };
                    if goto_next_lookat {
                        Some(self.get_next_look_at_or_processing(*context.cycle_time))
                    } else {
                        None
                    }
                })
            }
            CalibrationState::Process => {
                // TODO Handle not enough measurements

                let solved_result = solve(
                    Corrections::default(),
                    self.current_measurements.clone(),
                    *context.field_dimensions,
                );

                self.last_calibration_corrections = Some(solved_result);
                Some(CalibrationState::Finish)
            }
            CalibrationState::Finish => None,
        }
    }

    fn get_next_look_at_or_processing(&mut self, dispatch_time: CycleTime) -> CalibrationState {
        self.get_next_look_at()
            .map_or(CalibrationState::Process, |(target, camera)| {
                CalibrationState::LookAt {
                    camera,
                    target,
                    dispatch_time,
                }
            })
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
            original_dispatch_time.start_time == measurement.dispatch_time.start_time
        })
        .cloned()
        .cloned()
}

// TODO Add fancier logic to either set this via parameters OR detect the location, walk, etc
fn generate_look_at_list() -> Vec<(Point2<Ground>, CameraPosition)> {
    let look_at_points: Vec<Point2<Ground>> = vec![
        point![1.0, 0.0],
        point![1.0, -0.5],
        point![3.0, -0.5],
        point![3.0, 0.0],
        point![3.0, 0.5],
        point![1.0, -0.5],
    ];

    look_at_points
        .iter()
        .map(|&point| (point, CameraPosition::Top))
        .collect()
}
