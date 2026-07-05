use std::{path::PathBuf, time::Duration};

use kinematics::joints::head::HeadJoints;
use ros_z::Message;
use serde::{Deserialize, Serialize};
use types::{
    gamepad::GamepadManualMotion,
    motion_command::{HeadMotion, MotionCommand},
};

#[derive(Debug, Clone, Serialize, Deserialize, Message)]
#[serde(deny_unknown_fields)]
pub struct Parameters {
    pub device_path: PathBuf,
    pub reconnect_interval: Duration,
    pub deadzone: f32,
    pub maximum_forward_velocity: f32,
    pub maximum_left_velocity: f32,
    pub maximum_angular_velocity: f32,
    pub head_center: HeadJoints<f32>,
    pub head_minimum: HeadJoints<f32>,
    pub head_maximum: HeadJoints<f32>,
    pub head_velocity: HeadJoints<f32>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Axes {
    left_x: f32,
    left_y: f32,
    right_x: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Dpad {
    x: i32,
    y: i32,
}

#[derive(Debug, Clone)]
pub struct GamepadState {
    active: bool,
    last_start_pressed: bool,
    axes: Axes,
    dpad: Dpad,
    manual_head: HeadJoints<f32>,
}

pub fn normalize_axis(value: i32, minimum: i32, maximum: i32, deadzone: f32) -> f32 {
    let half_range = (maximum - minimum) as f32 / 2.0;
    if half_range <= 0.0 {
        return 0.0;
    }

    let center = (minimum + maximum) as f32 / 2.0;
    let normalized = ((value as f32 - center) / half_range).clamp(-1.0, 1.0);
    if normalized.abs() < deadzone {
        0.0
    } else {
        normalized
    }
}

impl GamepadState {
    pub fn new(parameters: &Parameters) -> Self {
        Self {
            active: false,
            last_start_pressed: false,
            axes: Axes::default(),
            dpad: Dpad::default(),
            manual_head: parameters.head_center,
        }
    }

    pub fn update_start_button(&mut self, pressed: bool, parameters: &Parameters) {
        if pressed && !self.last_start_pressed {
            self.active = !self.active;
            tracing::info!(
                target: "gamepad::input",
                active = self.active,
                "gamepad manual control toggled"
            );
            if !self.active {
                self.manual_head = parameters.head_center;
            }
        }
        self.last_start_pressed = pressed;
    }

    pub fn set_left_x(&mut self, value: f32) {
        self.axes.left_x = value.clamp(-1.0, 1.0);
    }

    pub fn set_left_y(&mut self, value: f32) {
        self.axes.left_y = value.clamp(-1.0, 1.0);
    }

    pub fn set_right_x(&mut self, value: f32) {
        self.axes.right_x = value.clamp(-1.0, 1.0);
    }

    pub fn set_dpad_x(&mut self, value: i32) {
        self.dpad.x = value.clamp(-1, 1);
    }

    pub fn set_dpad_y(&mut self, value: i32) {
        self.dpad.y = value.clamp(-1, 1);
    }

    pub fn manual_motion(
        &mut self,
        parameters: &Parameters,
        cycle_duration: Duration,
        input_is_unavailable: bool,
    ) -> GamepadManualMotion {
        if input_is_unavailable {
            self.axes = Axes::default();
        }

        if self.active && !input_is_unavailable {
            let seconds = cycle_duration.as_secs_f32();
            self.manual_head.yaw += self.dpad.x as f32 * parameters.head_velocity.yaw * seconds;
            self.manual_head.pitch += self.dpad.y as f32 * parameters.head_velocity.pitch * seconds;
        }

        self.manual_head.yaw = self
            .manual_head
            .yaw
            .clamp(parameters.head_minimum.yaw, parameters.head_maximum.yaw);
        self.manual_head.pitch = self
            .manual_head
            .pitch
            .clamp(parameters.head_minimum.pitch, parameters.head_maximum.pitch);

        let (forward, left, angular_velocity) = if self.active && !input_is_unavailable {
            (
                -self.axes.left_y * parameters.maximum_forward_velocity,
                -self.axes.left_x * parameters.maximum_left_velocity,
                -self.axes.right_x * parameters.maximum_angular_velocity,
            )
        } else {
            (0.0, 0.0, 0.0)
        };

        GamepadManualMotion {
            active: self.active,
            motion_command: MotionCommand::WalkWithVelocity {
                head: HeadMotion::JointAngles {
                    joints: self.manual_head,
                },
                velocity: linear_algebra::vector![forward, left],
                angular_velocity,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use types::motion_command::{HeadMotion, MotionCommand};

    use super::*;

    fn parameters() -> Parameters {
        Parameters {
            device_path: PathBuf::from("/dev/input/by-id/test-event-joystick"),
            reconnect_interval: Duration::from_secs(1),
            deadzone: 0.15,
            maximum_forward_velocity: 0.3,
            maximum_left_velocity: 0.2,
            maximum_angular_velocity: 0.5,
            head_center: HeadJoints {
                yaw: 0.0,
                pitch: 0.4,
            },
            head_minimum: HeadJoints {
                yaw: -0.785,
                pitch: -0.3,
            },
            head_maximum: HeadJoints {
                yaw: 0.785,
                pitch: 1.0,
            },
            head_velocity: HeadJoints {
                yaw: 1.0,
                pitch: 1.0,
            },
        }
    }

    fn walk_motion(message: GamepadManualMotion) -> MotionCommand {
        message.motion_command
    }

    #[test]
    fn normalizes_center_deadzone_and_extremes() {
        assert_eq!(normalize_axis(128, 0, 255, 0.15), 0.0);
        assert_eq!(normalize_axis(140, 0, 255, 0.15), 0.0);
        assert!(normalize_axis(255, 0, 255, 0.15) > 0.99);
        assert!(normalize_axis(0, 0, 255, 0.15) < -0.99);
        assert_eq!(normalize_axis(10, 10, 10, 0.15), 0.0);
    }

    #[test]
    fn start_button_rising_edge_toggles_once() {
        let parameters = parameters();
        let mut state = GamepadState::new(&parameters);

        state.update_start_button(true, &parameters);
        assert!(
            state
                .manual_motion(&parameters, Duration::ZERO, false)
                .active
        );

        state.update_start_button(true, &parameters);
        assert!(
            state
                .manual_motion(&parameters, Duration::ZERO, false)
                .active
        );

        state.update_start_button(false, &parameters);
        state.update_start_button(true, &parameters);
        assert!(
            !state
                .manual_motion(&parameters, Duration::ZERO, false)
                .active
        );
    }

    #[test]
    fn unavailable_input_keeps_active_but_zeroes_velocity() {
        let parameters = parameters();
        let mut state = GamepadState::new(&parameters);

        state.update_start_button(true, &parameters);
        state.set_left_y(-1.0);
        state.set_left_x(-1.0);
        state.set_right_x(-1.0);

        let message = state.manual_motion(&parameters, Duration::from_millis(20), true);

        assert!(message.active);
        assert_eq!(
            walk_motion(message),
            MotionCommand::WalkWithVelocity {
                head: HeadMotion::JointAngles {
                    joints: parameters.head_center,
                },
                velocity: linear_algebra::vector![0.0, 0.0],
                angular_velocity: 0.0,
            }
        );
    }

    #[test]
    fn reconnect_without_axis_event_does_not_resume_old_velocity() {
        let parameters = parameters();
        let mut state = GamepadState::new(&parameters);

        state.update_start_button(true, &parameters);
        state.set_left_y(-1.0);

        state.manual_motion(&parameters, Duration::from_millis(20), true);

        let message = state.manual_motion(&parameters, Duration::from_millis(20), false);
        assert_eq!(
            walk_motion(message),
            MotionCommand::WalkWithVelocity {
                head: HeadMotion::JointAngles {
                    joints: parameters.head_center,
                },
                velocity: linear_algebra::vector![0.0, 0.0],
                angular_velocity: 0.0,
            }
        );

        state.set_left_y(-1.0);

        let message = state.manual_motion(&parameters, Duration::from_millis(20), false);
        assert_eq!(
            walk_motion(message),
            MotionCommand::WalkWithVelocity {
                head: HeadMotion::JointAngles {
                    joints: parameters.head_center,
                },
                velocity: linear_algebra::vector![0.3, 0.0],
                angular_velocity: 0.0,
            }
        );
    }

    #[test]
    fn axes_map_to_forward_left_and_turn() {
        let parameters = parameters();
        let mut state = GamepadState::new(&parameters);

        state.update_start_button(true, &parameters);
        state.set_left_y(-1.0);
        state.set_left_x(-1.0);
        state.set_right_x(-1.0);

        let message = state.manual_motion(&parameters, Duration::from_millis(20), false);

        assert_eq!(
            walk_motion(message),
            MotionCommand::WalkWithVelocity {
                head: HeadMotion::JointAngles {
                    joints: parameters.head_center,
                },
                velocity: linear_algebra::vector![0.3, 0.2],
                angular_velocity: 0.5,
            }
        );
    }

    #[test]
    fn dpad_integrates_head_target_and_clamps_without_wrapping() {
        let parameters = parameters();
        let mut state = GamepadState::new(&parameters);

        state.update_start_button(true, &parameters);
        state.set_dpad_x(-1);
        state.set_dpad_y(-1);

        let message = state.manual_motion(&parameters, Duration::from_secs(10), false);

        assert_eq!(
            walk_motion(message),
            MotionCommand::WalkWithVelocity {
                head: HeadMotion::JointAngles {
                    joints: HeadJoints {
                        yaw: parameters.head_minimum.yaw,
                        pitch: parameters.head_minimum.pitch,
                    },
                },
                velocity: linear_algebra::vector![0.0, 0.0],
                angular_velocity: 0.0,
            }
        );
    }

    #[test]
    fn toggle_off_resets_manual_head_target_to_center() {
        let parameters = parameters();
        let mut state = GamepadState::new(&parameters);

        state.update_start_button(true, &parameters);
        state.set_dpad_x(1);
        state.manual_motion(&parameters, Duration::from_secs(1), false);
        state.update_start_button(false, &parameters);
        state.update_start_button(true, &parameters);

        let message = state.manual_motion(&parameters, Duration::ZERO, false);

        assert!(!message.active);
        assert_eq!(
            walk_motion(message),
            MotionCommand::WalkWithVelocity {
                head: HeadMotion::JointAngles {
                    joints: parameters.head_center,
                },
                velocity: linear_algebra::vector![0.0, 0.0],
                angular_velocity: 0.0,
            }
        );
    }
}
