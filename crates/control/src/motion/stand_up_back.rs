use color_eyre::Result;
use context_attribute::context;
use filtering::low_pass_filter::LowPassFilter;
use framework::MainOutput;
use motionfile::{MotionFile, MotionInterpolator, SplineInterpolator, TimedSpline};
use nalgebra::Vector2;
use types::{
    ConditionInput, CycleTime, Facing, Joints, JointsVelocity, MotionCommand, MotionSafeExits,
    MotionSelection, MotionType, SensorData,
};

pub struct StandUpBack {
    dispatch_to_initial: Option<SplineInterpolator<Joints<f32>>>,
    last_motion_type: MotionType,
    interpolator: MotionInterpolator<Joints<f32>>,
    filtered_gyro: LowPassFilter<Vector2<f32>>,
}

#[context]
pub struct CreationContext {
    pub gyro_low_pass_filter_coefficient:
        Parameter<f32, "stand_up.gyro_low_pass_filter_coefficient">,
    pub gyro_low_pass_filter_tolerance: Parameter<f32, "stand_up.gyro_low_pass_filter_tolerance">,

    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,
}

#[context]
pub struct CycleContext {
    pub condition_input: Input<ConditionInput, "condition_input">,
    pub cycle_time: Input<CycleTime, "cycle_time">,
    pub motion_command: Input<MotionCommand, "motion_command">,
    pub motion_selection: Input<MotionSelection, "motion_selection">,
    pub sensor_data: Input<SensorData, "sensor_data">,

    pub gyro_low_pass_filter_coefficient:
        Parameter<f32, "stand_up.gyro_low_pass_filter_coefficient">,
    pub gyro_low_pass_filter_tolerance: Parameter<f32, "stand_up.gyro_low_pass_filter_tolerance">,
    pub maximum_velocity: Parameter<JointsVelocity, "maximum_joint_velocities">,

    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub stand_up_back_positions: MainOutput<Joints<f32>>,
}

impl StandUpBack {
    pub fn new(context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_motion_type: Default::default(),
            dispatch_to_initial: None,
            interpolator: MotionFile::from_path("etc/motions/stand_up_back_dortmund_2022.json")?
                .try_into()?,
            filtered_gyro: LowPassFilter::with_smoothing_factor(
                Vector2::zeros(),
                *context.gyro_low_pass_filter_coefficient,
            ),
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let last_cycle_duration = context.cycle_time.last_cycle_duration;
        let angular_velocity = context
            .sensor_data
            .inertial_measurement_unit
            .angular_velocity;

        self.filtered_gyro
            .update(Vector2::new(angular_velocity.x, angular_velocity.y));

        match (
            self.last_motion_type,
            context.motion_selection.current_motion,
        ) {
            (MotionType::StandUpBack, MotionType::StandUpBack) => {
                if let Some(true) = self
                    .dispatch_to_initial
                    .as_ref()
                    .map(|interpolator| interpolator.is_finished())
                {
                    self.dispatch_to_initial = None;
                }
                if let Some(dispatch) = self.dispatch_to_initial.as_mut() {
                    dispatch.advance_by(last_cycle_duration);
                } else {
                    self.interpolator
                        .advance_by(last_cycle_duration, context.condition_input);
                }
            }
            (_, _) => {
                self.dispatch_to_initial = None;
                self.interpolator.reset();
            }
        }
        self.last_motion_type = context.motion_selection.current_motion;

        context.motion_safe_exits[MotionType::StandUpBack] = false;
        if self.interpolator.is_finished() {
            match context.motion_command {
                MotionCommand::StandUp { facing: Facing::Up } => {
                    self.dispatch_to_initial = Some(
                        TimedSpline::try_new_transition_with_velocity(
                            context.sensor_data.positions,
                            self.interpolator.initial_positions(),
                            *context.maximum_velocity,
                        )?
                        .into(),
                    );
                    self.interpolator.reset()
                }
                _ => {
                    self.dispatch_to_initial = None;
                    context.motion_safe_exits[MotionType::StandUpBack] = true;
                }
            };
        }

        if let Some(dispatch) = self.dispatch_to_initial.as_ref() {
            Ok(MainOutputs {
                stand_up_back_positions: dispatch.value().into(),
            })
        } else {
            Ok(MainOutputs {
                stand_up_back_positions: self.interpolator.value().into(),
            })
        }
    }
}
