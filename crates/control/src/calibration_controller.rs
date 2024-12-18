use std::{time::Duration, vec};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use calibration::{
    corrections::Corrections,
    goal_box::{measurement::Measurement, residuals::GoalBoxResiduals},
    solve,
};
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
    inner_states: StateTracking,
    corrections: Option<Corrections>,
    look_at_list: Vec<(Point2<Ground>, CameraPosition)>,
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
    look_at_dispatch_delay: Parameter<Duration, "calibration_controller.look_at_dispatch_delay">,
    stabilization_delay: Parameter<Duration, "calibration_controller.stabilization_delay">,
    max_retries_per_capture: Parameter<u32, "calibration_controller.max_retries_per_capture">,

    calibration_measurements: AdditionalOutput<Vec<Measurement>, "calibration_inner.measurements">,
    last_calibration_corrections: AdditionalOutput<Corrections, "last_calibration_corrections">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub calibration_command: MainOutput<Option<CalibrationCommand>>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct StateTracking {
    look_at_index: usize,
    calibration_state: CalibrationState,
    measurements: Vec<Measurement>,
    last_capture_retries: u32,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
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
        target: Point2<Ground>,
        camera: CameraPosition,
        dispatch_time: CycleTime,
    },
    Finish,
}

impl CalibrationState {
    fn as_calibration_command(&self) -> Option<CalibrationCommand> {
        match *self {
            CalibrationState::LookAt {
                target,
                camera,
                dispatch_time,
            } => Some(CalibrationCommand {
                target,
                camera,
                dispatch_time,
                capture: false,
            }),
            CalibrationState::Capture {
                target,
                camera,
                dispatch_time,
            } => Some(CalibrationCommand {
                target,
                camera,
                dispatch_time,
                capture: true,
            }),
            _ => None,
        }
    }
}

impl CalibrationController {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            inner_states: Default::default(),
            look_at_list: generate_look_at_list(),
            corrections: None,
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        if *context.primary_state != PrimaryState::Calibration {
            self.inner_states.calibration_state = CalibrationState::Inactive;
            return Ok(MainOutputs::default());
        }

        match self.inner_states.calibration_state {
            CalibrationState::Inactive => {
                self.inner_states = Default::default();
                self.inner_states.calibration_state = CalibrationState::Initialize {
                    started_time: *context.cycle_time,
                };
            }
            CalibrationState::Initialize {
                started_time: activated_time,
            } => {
                let waiting_duration = context
                    .cycle_time
                    .start_time
                    .duration_since(activated_time.start_time)
                    .unwrap_or_default();

                if waiting_duration >= *context.stabilization_delay {
                    self.inner_states.calibration_state = self
                        .get_next_look_at(*context.cycle_time)
                        .unwrap_or(CalibrationState::Finish);
                }
            }
            CalibrationState::LookAt {
                dispatch_time,
                camera,
                target,
            } => {
                let time_diff = context
                    .cycle_time
                    .start_time
                    .duration_since(dispatch_time.start_time)
                    .unwrap_or_default();

                if time_diff > *context.look_at_dispatch_delay {
                    self.inner_states.last_capture_retries = 0;
                    self.inner_states.calibration_state = CalibrationState::Capture {
                        target,
                        camera,
                        dispatch_time: *context.cycle_time,
                    };
                }
            }
            CalibrationState::Capture {
                dispatch_time,
                camera,
                ..
            } => {
                self.process_capture(camera, &context, dispatch_time);
            }
            CalibrationState::Finish => {}
        };

        context
            .calibration_measurements
            .fill_if_subscribed(|| self.inner_states.measurements.clone());

        context
            .last_calibration_corrections
            .mutate_if_subscribed(|data| {
                if let Some(corrections) = self.corrections {
                    data.replace(corrections);
                } else {
                    data.take();
                }
            });

        Ok(MainOutputs {
            calibration_command: self
                .inner_states
                .calibration_state
                .as_calibration_command()
                .into(),
        })
    }

    fn process_capture(
        &mut self,
        camera: CameraPosition,
        context: &CycleContext,
        dispatch_time: CycleTime,
    ) {
        let calibration_response = collect_filtered_values(
            match camera {
                CameraPosition::Top => &context.measurement_top,
                CameraPosition::Bottom => &context.measurement_bottom,
            },
            &dispatch_time,
        );

        let goto_next_lookat = calibration_response.map_or(false, |response| {
            if let Some(measurement) = response.measurement {
                self.inner_states.measurements.push(measurement);
                true
            } else {
                self.inner_states.last_capture_retries += 1;
                self.inner_states.last_capture_retries > *context.max_retries_per_capture
            }
        });
        if goto_next_lookat {
            self.inner_states.calibration_state = self
                .get_next_look_at(*context.cycle_time)
                .unwrap_or_else(|| self.calibrate(context));
        }
    }

    fn calibrate(&mut self, context: &CycleContext) -> CalibrationState {
        // TODO Handle not enough inner.measurements
        let solved_result = solve::<GoalBoxResiduals>(
            Corrections::default(),
            self.inner_states.measurements.clone(),
            *context.field_dimensions,
        );

        self.corrections = Some(solved_result);
        CalibrationState::Finish
    }

    fn get_next_look_at(&mut self, dispatch_time: CycleTime) -> Option<CalibrationState> {
        let index = self.inner_states.look_at_index;
        self.inner_states.look_at_index += 1;
        self.look_at_list
            .get(index)
            .copied()
            .map(|(target, camera)| CalibrationState::LookAt {
                camera,
                target,
                dispatch_time,
            })
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
