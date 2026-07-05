use std::{io, path::Path};

use evdev::{AbsoluteAxisCode, Device, EventSummary, KeyCode};

use crate::state::{GamepadState, normalize_axis};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    LeftX,
    LeftY,
    RightX,
    HatX,
    HatY,
}

#[derive(Debug, Clone, Copy)]
struct AxisRange {
    minimum: i32,
    maximum: i32,
}

impl AxisRange {
    const SONY_STICK: Self = Self {
        minimum: 0,
        maximum: 255,
    };

    fn normalize(self, value: i32, deadzone: f32) -> f32 {
        normalize_axis(value, self.minimum, self.maximum, deadzone)
    }
}

#[derive(Debug, Clone, Copy)]
struct AxisRanges {
    left_x: AxisRange,
    left_y: AxisRange,
    right_x: AxisRange,
}

impl Default for AxisRanges {
    fn default() -> Self {
        Self {
            left_x: AxisRange::SONY_STICK,
            left_y: AxisRange::SONY_STICK,
            right_x: AxisRange::SONY_STICK,
        }
    }
}

pub struct GamepadReader {
    device: Device,
    axis_ranges: AxisRanges,
}

pub fn axis_for(axis: AbsoluteAxisCode) -> Option<Axis> {
    match axis {
        AbsoluteAxisCode::ABS_X => Some(Axis::LeftX),
        AbsoluteAxisCode::ABS_Y => Some(Axis::LeftY),
        AbsoluteAxisCode::ABS_RX => Some(Axis::RightX),
        AbsoluteAxisCode::ABS_HAT0X => Some(Axis::HatX),
        AbsoluteAxisCode::ABS_HAT0Y => Some(Axis::HatY),
        _ => None,
    }
}

pub fn is_start_button(key: KeyCode) -> bool {
    key == KeyCode::BTN_START || key == KeyCode::BTN_MODE
}

impl GamepadReader {
    pub fn open(path: &Path) -> io::Result<Self> {
        let device = Device::open(path)?;
        device.set_nonblocking(true)?;
        let axis_ranges = read_axis_ranges(&device)?;

        let supported_keys = device.supported_keys();
        let supported_axes = device.supported_absolute_axes();
        tracing::info!(
            target: "gamepad::evdev",
            path = %path.display(),
            name = ?device.name(),
            physical_path = ?device.physical_path(),
            unique_name = ?device.unique_name(),
            supports_start = supported_keys.is_some_and(|keys| keys.contains(KeyCode::BTN_START)),
            supports_mode = supported_keys.is_some_and(|keys| keys.contains(KeyCode::BTN_MODE)),
            supports_left_x = supported_axes.is_some_and(|axes| axes.contains(AbsoluteAxisCode::ABS_X)),
            supports_left_y = supported_axes.is_some_and(|axes| axes.contains(AbsoluteAxisCode::ABS_Y)),
            supports_right_x = supported_axes.is_some_and(|axes| axes.contains(AbsoluteAxisCode::ABS_RX)),
            supports_hat_x = supported_axes.is_some_and(|axes| axes.contains(AbsoluteAxisCode::ABS_HAT0X)),
            supports_hat_y = supported_axes.is_some_and(|axes| axes.contains(AbsoluteAxisCode::ABS_HAT0Y)),
            ?axis_ranges,
            "opened gamepad device"
        );

        Ok(Self {
            device,
            axis_ranges,
        })
    }

    pub fn drain_events(
        &mut self,
        state: &mut GamepadState,
        parameters: &crate::Parameters,
    ) -> io::Result<()> {
        loop {
            let events = match self.device.fetch_events() {
                Ok(events) => events,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(()),
                Err(error) => return Err(error),
            };

            let mut saw_event = false;
            for event in events {
                saw_event = true;
                match event.destructure() {
                    EventSummary::Key(_, key, value) if is_start_button(key) => {
                        tracing::debug!(
                            target: "gamepad::evdev",
                            ?key,
                            value,
                            mapped = "start",
                            "received gamepad key event"
                        );
                        state.update_start_button(value != 0, parameters)
                    }
                    EventSummary::Key(_, key, value) => {
                        tracing::debug!(
                            target: "gamepad::evdev",
                            ?key,
                            value,
                            "received unmapped gamepad key event"
                        );
                    }
                    EventSummary::AbsoluteAxis(_, axis, value) => {
                        let mapped_axis = axis_for(axis);
                        let normalized_value = match mapped_axis {
                            Some(Axis::LeftX) => Some(
                                self.axis_ranges
                                    .left_x
                                    .normalize(value, parameters.deadzone),
                            ),
                            Some(Axis::LeftY) => Some(
                                self.axis_ranges
                                    .left_y
                                    .normalize(value, parameters.deadzone),
                            ),
                            Some(Axis::RightX) => Some(
                                self.axis_ranges
                                    .right_x
                                    .normalize(value, parameters.deadzone),
                            ),
                            Some(Axis::HatX) | Some(Axis::HatY) => {
                                Some(value.signum().clamp(-1, 1) as f32)
                            }
                            None => None,
                        };
                        tracing::debug!(
                            target: "gamepad::evdev",
                            ?axis,
                            value,
                            ?mapped_axis,
                            ?normalized_value,
                            "received gamepad axis event"
                        );
                        apply_axis_event(state, self.axis_ranges, axis, value, parameters.deadzone);
                    }
                    other => {
                        tracing::debug!(
                            target: "gamepad::evdev",
                            ?other,
                            "received unmapped gamepad event"
                        );
                    }
                }
            }

            if !saw_event {
                return Ok(());
            }
        }
    }
}

fn read_axis_ranges(device: &Device) -> io::Result<AxisRanges> {
    let mut axis_ranges = AxisRanges::default();
    for (axis, info) in device.get_absinfo()? {
        let range = AxisRange {
            minimum: info.minimum(),
            maximum: info.maximum(),
        };
        match axis_for(axis) {
            Some(Axis::LeftX) => axis_ranges.left_x = range,
            Some(Axis::LeftY) => axis_ranges.left_y = range,
            Some(Axis::RightX) => axis_ranges.right_x = range,
            Some(Axis::HatX) | Some(Axis::HatY) | None => {}
        }
    }
    Ok(axis_ranges)
}

fn apply_axis_event(
    state: &mut GamepadState,
    axis_ranges: AxisRanges,
    axis: AbsoluteAxisCode,
    value: i32,
    deadzone: f32,
) {
    match axis_for(axis) {
        Some(Axis::LeftX) => state.set_left_x(axis_ranges.left_x.normalize(value, deadzone)),
        Some(Axis::LeftY) => state.set_left_y(axis_ranges.left_y.normalize(value, deadzone)),
        Some(Axis::RightX) => state.set_right_x(axis_ranges.right_x.normalize(value, deadzone)),
        Some(Axis::HatX) => state.set_dpad_x(value.signum().clamp(-1, 1)),
        Some(Axis::HatY) => state.set_dpad_y(value.signum().clamp(-1, 1)),
        None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_sony_controller_axes() {
        assert_eq!(axis_for(AbsoluteAxisCode::ABS_X), Some(Axis::LeftX));
        assert_eq!(axis_for(AbsoluteAxisCode::ABS_Y), Some(Axis::LeftY));
        assert_eq!(axis_for(AbsoluteAxisCode::ABS_RX), Some(Axis::RightX));
        assert_eq!(axis_for(AbsoluteAxisCode::ABS_HAT0X), Some(Axis::HatX));
        assert_eq!(axis_for(AbsoluteAxisCode::ABS_HAT0Y), Some(Axis::HatY));
        assert_eq!(axis_for(AbsoluteAxisCode::ABS_Z), None);
    }

    #[test]
    fn maps_start_button_only() {
        assert!(is_start_button(KeyCode::BTN_START));
        assert!(is_start_button(KeyCode::BTN_MODE));
        assert!(!is_start_button(KeyCode::BTN_SELECT));
    }
}
