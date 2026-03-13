use std::time::{Duration, SystemTime};

use booster::Kick;
use booster_sdk::types::RobotMode;
use color_eyre::Result;
use context_attribute::context;
use hardware::{HighLevelInterface, MotionRuntimeInteface, VisualKickInterface};
use ros2::std_msgs::header::Header;
use serde::{Deserialize, Serialize};
use types::{cycle_time::CycleTime, motion_command::MotionCommand, motion_runtime::MotionRuntime};

#[derive(Deserialize, Serialize)]
pub struct BoosterKick {
    pub last_motion_command: Option<MotionCommand>,

    pub last_kick_time: SystemTime,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    robot_mode: RequiredInput<Option<RobotMode>, "WorldState", "robot_mode?">,

    motion_command: Input<MotionCommand, "WorldState", "motion_command">,
    cycle_time: Input<CycleTime, "cycle_time">,

    kick_message_interval: Parameter<Duration, "kicking.kick_message_interval">,

    hardware_interface: HardwareInterface,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {}

impl BoosterKick {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_motion_command: None,
            last_kick_time: SystemTime::UNIX_EPOCH,
        })
    }

    pub fn cycle(
        &mut self,
        context: CycleContext<
            impl HighLevelInterface + MotionRuntimeInteface + VisualKickInterface,
        >,
    ) -> Result<MainOutputs> {
        if !matches!(
            context.hardware_interface.get_motion_runtime_type()?,
            MotionRuntime::Booster
        ) | !matches!(context.robot_mode, RobotMode::Walking)
        {
            return Ok(MainOutputs {});
        }

        match context.motion_command {
            MotionCommand::VisualKick {
                ball_position,
                kick_direction,
                target_position,
                robot_theta_to_field,
                kick_power,
                ..
            } => {
                if !matches!(
                    self.last_motion_command,
                    Some(MotionCommand::VisualKick { .. })
                ) {
                    set_visual_kick_activation_state(&context, true);
                }

                let kick = Kick {
                    header: Header {
                        stamp: context.cycle_time.start_time.into(),
                        frame_id: "".to_string(),
                    },
                    ball_position_x: ball_position.x() as f64,
                    ball_position_y: ball_position.y() as f64,
                    kick_direction_angle: kick_direction.angle() as f64,
                    target_position_x: target_position.x() as f64,
                    target_position_y: target_position.y() as f64,
                    robot_angle_to_field: robot_theta_to_field.angle() as f64,
                    kick_power: *kick_power,
                };

                if context
                    .cycle_time
                    .start_time
                    .duration_since(self.last_kick_time)
                    .expect("Time ran backwards")
                    > *context.kick_message_interval
                {
                    self.last_kick_time = context.cycle_time.start_time;
                    context.hardware_interface.write_visual_kick(kick)?;
                }
            }
            _ => set_visual_kick_activation_state(&context, false),
        };

        self.last_motion_command = Some(context.motion_command.clone());

        Ok(MainOutputs {})
    }
}

fn set_visual_kick_activation_state(context: &CycleContext<impl HighLevelInterface>, start: bool) {
    let _ = context
        .hardware_interface
        .visual_kick(start)
        .inspect_err(|err| log::error!("{err:?}"));
}
