use std::{boxed::Box, future::Future, pin::Pin, sync::Arc, time::Duration};

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use booster::{JointsMotorState, MotorState};
use filtering::low_pass_filter::LowPassFilter;
use kinematics::joints::{Joints, head::HeadJoints};
use ros_z::{prelude::*, time::Time};
use types::{
    motion_command::{HeadMotion, ImageRegion, MotionCommand},
    parameters::HeadMotionParameters,
};

const MOTION_COMMAND_TOPIC: &str = "behavior/motion_command";

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub parameters: HeadMotionParameters,
}

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("head_motion").build().await?;

    let parameters = node.bind_parameter_as::<Parameters>("head_motion")?;
    let look_around_target_joints_cache = node
        .subscriber::<HeadJoints<f32>>("look_around_target_joints")
        .cache(1)
        .build()
        .await?;
    let look_at_cache = node
        .subscriber::<HeadJoints<f32>>("look_at")
        .cache(1)
        .build()
        .await?;
    let motor_states_cache = node
        .subscriber::<Joints<MotorState>>("inputs/serial_motor_states")
        .cache(1)
        .build()
        .await?;
    let motion_command_cache = node
        .subscriber::<MotionCommand>(MOTION_COMMAND_TOPIC)
        .cache(1)
        .build()
        .await?;
    let head_joints_command_pub = node
        .publisher::<HeadJoints<f32>>("head_joints_command")
        .build()
        .await?;

    let mut state = HeadMotionState::new();
    let mut last_update = None;
    let mut tick = node.create_timer(Duration::from_millis(10));
    let default_motion_command = MotionCommand::default();

    loop {
        tick.tick().await;
        let now = node.clock().now();

        let Some(look_around_target_joints) = look_around_target_joints_cache
            .get_latest()
            .map(|joints| *joints)
        else {
            continue;
        };
        let Some(look_at) = look_at_cache.get_latest().map(|joints| *joints) else {
            continue;
        };
        let Some(motor_states) = motor_states_cache.get_latest() else {
            continue;
        };

        let motion_command = motion_command_cache.get_latest();
        let motion_command = motion_command.as_deref().unwrap_or(&default_motion_command);
        let last_cycle_duration = cycle_duration_since_last_update(&mut last_update, now);
        let parameters_snapshot = parameters.snapshot();
        let parameters = &parameters_snapshot.typed().parameters;
        let head_joints = state.update(
            parameters,
            look_around_target_joints,
            look_at,
            &motor_states,
            last_cycle_duration,
            motion_command,
        );

        head_joints_command_pub.publish(&head_joints).await?;
    }
}

fn cycle_duration_since_last_update(last_update: &mut Option<Time>, now: Time) -> Duration {
    let duration = last_update.map_or(Duration::ZERO, |last_update| {
        now.duration_since(last_update)
    });
    *last_update = Some(now);
    duration
}

#[derive(Default)]
struct HeadMotionState {
    last_positions: HeadJoints<f32>,
    lowpass_filter: LowPassFilter<HeadJoints<f32>>,
}

impl HeadMotionState {
    fn new() -> Self {
        Self {
            last_positions: Default::default(),
            lowpass_filter: LowPassFilter::with_smoothing_factor(Default::default(), 0.075),
        }
    }

    fn update(
        &mut self,
        parameters: &HeadMotionParameters,
        look_around_target_joints: HeadJoints<f32>,
        look_at: HeadJoints<f32>,
        motor_states: &Joints<MotorState>,
        last_cycle_duration: Duration,
        motion_command: &MotionCommand,
    ) -> HeadJoints<f32> {
        if let Some(injected_head_joints) = parameters.injected_head_joints {
            self.lowpass_filter.update(injected_head_joints);
            let filtered_head_joints = self.lowpass_filter.state();
            self.last_positions = filtered_head_joints;

            return filtered_head_joints;
        }

        let raw_positions = joints_from_motion(
            look_around_target_joints,
            look_at,
            motor_states,
            motion_command,
        );
        let maximum_movement = parameters.maximum_velocity * last_cycle_duration.as_secs_f32();

        let controlled_positions = HeadJoints {
            yaw: self.last_positions.yaw
                + (raw_positions.yaw - self.last_positions.yaw)
                    .clamp(-maximum_movement.yaw, maximum_movement.yaw),
            pitch: self.last_positions.pitch
                + (raw_positions.pitch - self.last_positions.pitch)
                    .clamp(-maximum_movement.pitch, maximum_movement.pitch),
        };

        let clamped_positions = HeadJoints {
            pitch: controlled_positions
                .pitch
                .clamp(parameters.minimum_pitch, parameters.maximum_pitch),
            yaw: controlled_positions
                .yaw
                .clamp(parameters.minimum_yaw, parameters.maximum_yaw),
        };

        self.last_positions = clamped_positions;
        clamped_positions
    }
}

fn joints_from_motion(
    look_around_target_joints: HeadJoints<f32>,
    look_at: HeadJoints<f32>,
    motor_states: &Joints<MotorState>,
    motion_command: &MotionCommand,
) -> HeadJoints<f32> {
    match motion_command.head_motion() {
        Some(HeadMotion::Center {
            image_region_target: ImageRegion::Top,
        }) => HeadJoints {
            yaw: 0.0,
            pitch: 0.4,
        },
        Some(HeadMotion::Center { .. }) => HeadJoints {
            yaw: 0.0,
            pitch: 0.4,
        },
        Some(HeadMotion::LookAt { .. }) | Some(HeadMotion::LookLeftAndRightOf { .. }) => look_at,
        Some(HeadMotion::Unstiff) => motor_states.positions().head,
        Some(HeadMotion::LookAround) | Some(HeadMotion::SearchForLostBall) => {
            look_around_target_joints
        }
        Some(_) | None => Default::default(),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use types::motion_command::{HeadMotion, ImageRegion};

    use super::*;

    #[test]
    fn center_head_motion_is_clamped() {
        let mut parameters = test_parameters();
        parameters.minimum_yaw = 0.1;
        parameters.maximum_yaw = 0.2;
        parameters.minimum_pitch = -0.2;
        parameters.maximum_pitch = 0.3;
        let mut state = HeadMotionState::new();

        let head_joints = state.update(
            &parameters,
            HeadJoints::default(),
            HeadJoints::default(),
            &Joints::default(),
            Duration::from_secs(1),
            &MotionCommand::Stand {
                head: HeadMotion::Center {
                    image_region_target: ImageRegion::Top,
                },
            },
        );

        assert_eq!(
            head_joints,
            HeadJoints {
                yaw: 0.1,
                pitch: 0.3,
            }
        );
    }

    #[test]
    fn unstiff_uses_measured_head_positions() {
        let measured_head = HeadJoints {
            yaw: 0.3,
            pitch: -0.2,
        };
        let motor_states = motor_states_with_head(measured_head);
        let mut state = HeadMotionState::new();

        let head_joints = state.update(
            &test_parameters(),
            HeadJoints::default(),
            HeadJoints::default(),
            &motor_states,
            Duration::from_secs(1),
            &MotionCommand::Stand {
                head: HeadMotion::Unstiff,
            },
        );

        assert_eq!(head_joints, measured_head);
    }

    #[test]
    fn first_complete_update_uses_zero_cycle_duration() {
        let start = Time::zero();
        let mut last_update = None;

        let first_duration =
            cycle_duration_since_last_update(&mut last_update, start + Duration::from_secs(3));
        let second_duration = cycle_duration_since_last_update(
            &mut last_update,
            start + Duration::from_secs(3) + Duration::from_millis(10),
        );

        assert_eq!(first_duration, Duration::ZERO);
        assert_eq!(second_duration, Duration::from_millis(10));
    }

    #[test]
    fn injected_head_joints_update_last_positions() {
        let mut injected_parameters = test_parameters();
        injected_parameters.maximum_velocity = HeadJoints::fill(0.0);
        injected_parameters.injected_head_joints = Some(HeadJoints {
            yaw: 1.0,
            pitch: -0.5,
        });
        let mut normal_parameters = test_parameters();
        normal_parameters.maximum_velocity = HeadJoints::fill(0.0);
        let mut state = HeadMotionState::new();

        let injected_output = state.update(
            &injected_parameters,
            HeadJoints::default(),
            HeadJoints::default(),
            &Joints::default(),
            Duration::from_secs(1),
            &MotionCommand::default(),
        );
        let after_injection = state.update(
            &normal_parameters,
            HeadJoints {
                yaw: 0.7,
                pitch: 0.2,
            },
            HeadJoints::default(),
            &Joints::default(),
            Duration::from_secs(1),
            &MotionCommand::Stand {
                head: HeadMotion::LookAround,
            },
        );

        assert_eq!(after_injection, injected_output);
    }

    fn test_parameters() -> HeadMotionParameters {
        HeadMotionParameters {
            maximum_pitch: 1.0,
            minimum_pitch: -1.0,
            maximum_velocity: HeadJoints {
                yaw: 100.0,
                pitch: 100.0,
            },
            maximum_defender_velocity: HeadJoints::default(),
            maximum_yaw: 1.0,
            minimum_yaw: -1.0,
            injected_head_joints: None,
        }
    }

    fn motor_states_with_head(head: HeadJoints<f32>) -> Joints<MotorState> {
        let mut motor_states = Joints::fill(MotorState::default());
        motor_states.head.yaw.position = head.yaw;
        motor_states.head.pitch.position = head.pitch;
        motor_states
    }
}
