use std::time::SystemTime;

use color_eyre::Result;
use context_attribute::context;
use framework::MainOutput;
use hardware::{PathsInterface, TimeInterface};
use motionfile::{MotionFile, MotionInterpolator};
use serde::{Deserialize, Serialize};
use types::{
    condition_input::ConditionInput,
    cycle_time::CycleTime,
    joints::Joints,
    motion_selection::{MotionSelection, MotionType},
    motor_commands::MotorCommands,
    sensor_data::SensorData,
};

#[derive(Deserialize, Serialize)]
pub struct Animation {
    saved_joint_values: Joints<f32>,
    wave_interpolator: MotionInterpolator<Joints<f32>>,
    into_sitdown_wave_interpolator: MotionInterpolator<Joints<f32>>,
    last_wave_finished_at: Option<SystemTime>,
}

#[context]
pub struct CreationContext {
    hardware_interface: HardwareInterface,
}

#[context]
pub struct CycleContext {
    cycle_time: Input<CycleTime, "cycle_time">,
    condition_input: Input<ConditionInput, "condition_input">,
    motion_selection: Input<MotionSelection, "motion_selection">,
    sensor_data: Input<SensorData, "sensor_data">,
    hardware_interface: HardwareInterface,
    time_between_waves: Parameter<u64, "time_between_waves">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub animation_commands: MainOutput<MotorCommands<Joints<f32>>>,
}

impl Animation {
    pub fn new(context: CreationContext<impl PathsInterface>) -> Result<Self> {
        let paths = context.hardware_interface.get_paths();
        let wave_interpolator =
            MotionFile::from_path(paths.motions.join("wave.json"))?.try_into()?;
        let into_sitdown_wave_interpolator =
            MotionFile::from_path(paths.motions.join("into_wave.json"))?.try_into()?;
        Ok(Self {
            saved_joint_values: Joints::default(),
            wave_interpolator,
            into_sitdown_wave_interpolator,
            last_wave_finished_at: None,
        })
    }

    pub fn cycle(&mut self, context: CycleContext<impl TimeInterface>) -> Result<MainOutputs> {
        let last_cycle_duration = context.cycle_time.last_cycle_duration;
        let motion_selection = context.motion_selection;
        let condition_input = context.condition_input;

        let wave_was_finished = self.wave_interpolator.is_finished();

        match context.motion_selection.current_motion {
            MotionType::AnimationStiff => {
                self.wave_interpolator
                    .advance_by(last_cycle_duration, condition_input);
            }
            MotionType::Animation => {
                self.into_sitdown_wave_interpolator
                    .advance_by(last_cycle_duration, condition_input);
            }
            _ => {
                self.into_sitdown_wave_interpolator
                    .set_initial_positions(context.sensor_data.positions);
                self.wave_interpolator.reset();
                self.into_sitdown_wave_interpolator.reset();
            }
        };

        let now = context.hardware_interface.get_now();
        if self.wave_interpolator.is_finished() && !wave_was_finished {
            self.last_wave_finished_at = Some(now);
        }

        if let Some(last_finish_time) = self.last_wave_finished_at {
            if now
                .duration_since(last_finish_time)
                .expect("time ran backwards")
                .as_secs()
                > *context.time_between_waves
            {
                self.wave_interpolator.reset();
                self.into_sitdown_wave_interpolator.reset();
                self.last_wave_finished_at = None;
            }
        }

        let output = MotorCommands {
            positions: if motion_selection.current_motion == MotionType::AnimationStiff {
                self.wave_interpolator.value()
            } else {
                self.into_sitdown_wave_interpolator.value()
            },
            stiffnesses: Joints::fill(0.3),
        };

        Ok(MainOutputs {
            animation_commands: output.into(),
        })
    }
}
