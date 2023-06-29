use std::time::{Duration, SystemTime, UNIX_EPOCH};

use approx::relative_eq;
use color_eyre::Result;
use context_attribute::context;
use filtering::low_pass_filter::LowPassFilter;
use framework::MainOutput;
use motionfile::{MotionFile, MotionInterpolator};
use nalgebra::Vector2;
use types::{
    configuration::FallProtection,
    configuration::FallStateEstimation as FallStateEstimationConfiguration, BodyJoints,
    ConditionInput, CycleTime, FallDirection, HeadJoints, Joints, JointsCommand, MotionCommand,
    MotionSelection, MotionType, SensorData,
};

pub struct FallProtector {
    start_time: SystemTime,
    interpolator: MotionInterpolator<Joints<f32>>,
    roll_pitch_filter: LowPassFilter<Vector2<f32>>,
}

#[context]
pub struct CreationContext {
    pub fall_protection: Parameter<FallProtection, "fall_protection">,
    pub fall_state_estimation: Parameter<FallStateEstimationConfiguration, "fall_state_estimation">,
}

#[context]
pub struct CycleContext {
    pub condition_input: Input<ConditionInput, "condition_input">,
    pub cycle_time: Input<CycleTime, "cycle_time">,
    pub motion_command: Input<MotionCommand, "motion_command">,
    pub motion_selection: Input<MotionSelection, "motion_selection">,
    pub sensor_data: Input<SensorData, "sensor_data">,

    pub fall_protection: Parameter<FallProtection, "fall_protection">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub fall_protection_command: MainOutput<JointsCommand<f32>>,
}

impl FallProtector {
    pub fn new(context: CreationContext) -> Result<Self> {
        Ok(Self {
            start_time: UNIX_EPOCH,
            interpolator: MotionFile::from_path("etc/motions/fall_back.json")?.try_into()?,
            roll_pitch_filter: LowPassFilter::with_smoothing_factor(
                Vector2::zeros(),
                context.fall_state_estimation.roll_pitch_low_pass_factor,
            ),
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let current_positions = context.sensor_data.positions;
        let mut head_stiffness = 1.0;
        let mut body_stiffness = 0.8;

        self.roll_pitch_filter
            .update(context.sensor_data.inertial_measurement_unit.roll_pitch);

        if context.motion_selection.current_motion != MotionType::FallProtection {
            self.start_time = context.cycle_time.start_time;
            return Ok(MainOutputs {
                fall_protection_command: JointsCommand {
                    positions: current_positions,
                    stiffnesses: Joints::fill(0.8),
                }
                .into(),
            });
        }

        if self.start_time.elapsed().unwrap() >= Duration::from_millis(500) {
            head_stiffness = 0.5;
        }

        match context.motion_command {
            MotionCommand::FallProtection {
                direction: FallDirection::Forward,
            } => {
                if relative_eq!(current_positions.head.pitch, -0.672, epsilon = 0.05)
                    && relative_eq!(current_positions.head.yaw.abs(), 0.0, epsilon = 0.05)
                {
                    head_stiffness = context.fall_protection.ground_impact_head_stiffness;
                }
            }
            MotionCommand::FallProtection { .. } => {
                if relative_eq!(current_positions.head.pitch, 0.5149, epsilon = 0.05)
                    && relative_eq!(current_positions.head.yaw.abs(), 0.0, epsilon = 0.05)
                {
                    head_stiffness = context.fall_protection.ground_impact_head_stiffness;
                }
            }
            _ => head_stiffness = context.fall_protection.ground_impact_head_stiffness,
        }

        let stiffnesses = Joints::from_head_and_body(
            HeadJoints::fill(head_stiffness),
            BodyJoints::fill_mirrored(
                context.fall_protection.arm_stiffness,
                context.fall_protection.leg_stiffness,
            ),
        );

        let fall_protection_command = match context.motion_command {
            MotionCommand::FallProtection {
                direction: FallDirection::Forward,
            } => {
                self.interpolator.reset();
                JointsCommand {
                    positions: Joints::from_head_and_body(
                        HeadJoints {
                            yaw: 0.0,
                            pitch: -0.672,
                        },
                        BodyJoints {
                            left_arm: context.fall_protection.left_arm_positions,
                            right_arm: context.fall_protection.right_arm_positions,
                            left_leg: current_positions.left_leg,
                            right_leg: current_positions.right_leg,
                        },
                    ),
                    stiffnesses,
                }
            }
            MotionCommand::FallProtection {
                direction: FallDirection::Backward,
            } => {
                self.interpolator.set_initial_positions(current_positions);
                self.interpolator.advance_by(
                    context.cycle_time.last_cycle_duration,
                    context.condition_input,
                );

                dbg!(self.roll_pitch_filter.state().y.abs());
                if self.roll_pitch_filter.state().y.abs()
                    > context.fall_protection.ground_impact_angular_threshold
                {
                    body_stiffness = context.fall_protection.ground_impact_body_stiffness;
                }
                let fall_back_stiffnesses = Joints::from_head_and_body(
                    HeadJoints::fill(head_stiffness),
                    BodyJoints::fill(body_stiffness),
                );
                JointsCommand {
                    positions: self.interpolator.value(),
                    stiffnesses: fall_back_stiffnesses,
                }
            }
            _ => {
                self.interpolator.reset();
                JointsCommand {
                    positions: Joints::from_head_and_body(
                        HeadJoints {
                            yaw: 0.0,
                            pitch: 0.5149,
                        },
                        BodyJoints {
                            left_arm: context.fall_protection.left_arm_positions,
                            right_arm: context.fall_protection.right_arm_positions,
                            left_leg: current_positions.left_leg,
                            right_leg: current_positions.right_leg,
                        },
                    ),
                    stiffnesses,
                }
            }
        };

        Ok(MainOutputs {
            fall_protection_command: fall_protection_command.into(),
        })
    }
}
