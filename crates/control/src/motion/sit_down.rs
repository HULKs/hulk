use color_eyre::{eyre::Context, Result};
use context_attribute::context;
use framework::MainOutput;
use types::{
    CycleTime, Joints, JointsCommand, MotionFile, MotionSafeExits, MotionSelection, MotionType,
    SensorData,
};

use crate::spline_interpolator::SplineInterpolator;

pub struct SitDown {
    interpolator: SplineInterpolator<Joints>,
}

#[context]
pub struct CreationContext {
    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,
}

#[context]
pub struct CycleContext {
    pub motion_selection: Input<MotionSelection, "motion_selection">,
    pub sensor_data: Input<SensorData, "sensor_data">,
    pub cycle_time: Input<CycleTime, "cycle_time">,

    pub motion_safe_exits: PersistentState<MotionSafeExits, "motion_safe_exits">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub sit_down_joints_command: MainOutput<JointsCommand<f32>>,
}

impl SitDown {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            interpolator: MotionFile::from_path("etc/motions/sit_down.json")?.try_into()?,
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let last_cycle_duration = context.cycle_time.last_cycle_duration;
        if context.motion_selection.current_motion == MotionType::SitDown {
            self.interpolator.advance_by(last_cycle_duration);
        } else {
            self.interpolator.reset();
        }

        context.motion_safe_exits[MotionType::SitDown] = self.interpolator.is_finished();

        Ok(MainOutputs {
            sit_down_joints_command: JointsCommand {
                positions: self
                    .interpolator
                    .value()
                    .wrap_err("error computing interpolation in sit_down")?,
                stiffnesses: Joints::fill(0.8),
            }
            .into(),
        })
    }
}
