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
    camera_position::CameraPosition, cycle_time::CycleTime, field_dimensions::FieldDimensions,
    primary_state::PrimaryState, world_state::CalibrationCommand,
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
    measurement_bottom:
        PerceptionInput<Option<Measurement>, "VisionBottom", "calibration_measurement?">,
    measurement_top: PerceptionInput<Option<Measurement>, "VisionTop", "calibration_measurement?">,
    cycle_time_top: PerceptionInput<CycleTime, "VisionTop", "cycle_time">,
    cycle_time_bottom: PerceptionInput<CycleTime, "VisionBottom", "cycle_time">,
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
                let mut values_top = collect_filtered_values(
                    &context.measurement_top,
                    &context.cycle_time_top,
                    &dispatch_time,
                );
                let mut values_bottom = collect_filtered_values(
                    &context.measurement_bottom,
                    &context.cycle_time_bottom,
                    &dispatch_time,
                );

                if !values_bottom.is_empty() || !values_bottom.is_empty() {
                    // TODO Require both cameras to give values or another mechanism to track that.
                    self.current_measurements.append(&mut values_top);
                    self.current_measurements.append(&mut values_bottom);

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
    measurement_perception_input: &PerceptionInput<Vec<Option<&Measurement>>>,
    cycletimes_perception_input: &PerceptionInput<Vec<&CycleTime>>,
    dispatch_time: &CycleTime,
) -> Vec<Measurement> {
    measurement_perception_input
        .persistent
        .iter()
        .zip(cycletimes_perception_input.persistent.iter())
        .flat_map(
            |((measurement_timestamp, measurements), (cycle_time_timestamp, cycle_times))| {
                assert_eq!(measurement_timestamp, cycle_time_timestamp);
                measurements
                    .iter()
                    .zip(cycle_times)
                    .filter_map(|(measurement, &&timestamp)| {
                        if timestamp.start_time >= dispatch_time.start_time {
                            *measurement
                        } else {
                            None
                        }
                    })
            },
        )
        .map(|m| (*m).clone())
        .collect_vec()
}

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
