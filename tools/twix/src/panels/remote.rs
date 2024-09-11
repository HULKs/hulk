use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};

use communication::messages::TextOrBinary;
use eframe::egui::Widget;
use gilrs::{Axis, Button, Gamepad, GamepadId, Gilrs};
use serde_json::{json, Value};
use types::{joints::head::HeadJoints, step::Step};

use crate::{nao::Nao, panel::Panel};

pub struct RemotePanel {
    nao: Arc<Nao>,
    gilrs: Gilrs,
    active_gamepad: Option<GamepadId>,
    enabled: bool,
    last_update: SystemTime,
}

impl Panel for RemotePanel {
    const NAME: &'static str = "Remote";

    fn new(nao: Arc<Nao>, _value: Option<&Value>) -> Self {
        let gilrs = Gilrs::new().expect("could not initialize gamepad library");
        let active_gamepad = None;
        let enabled = false;

        Self {
            nao,
            gilrs,
            active_gamepad,
            enabled,
            last_update: SystemTime::now(),
        }
    }

    fn save(&self) -> Value {
        json!({})
    }
}

fn get_axis_value(gamepad: Gamepad, axis: Axis) -> Option<f32> {
    Some(gamepad.axis_data(axis)?.value())
}

impl RemotePanel {
    fn reset(&self) {
        self.update_step(Value::Null);
        self.update_look_at_angle(Value::Null);
    }

    fn update_step(&self, step: Value) {
        self.nao.write(
            "parameters.step_planner.injected_step",
            TextOrBinary::Text(step),
        )
    }

    fn update_look_at_angle(&self, joints: Value) {
        self.nao.write(
            "parameters.head_motion.injected_head_joints",
            TextOrBinary::Text(joints),
        )
    }
}

impl Widget for &mut RemotePanel {
    fn ui(self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        const UPDATE_DELAY: Duration = Duration::from_millis(100);
        const HEAD_PITCH_SCALE: f32 = 1.0;
        const HEAD_YAW_SCALE: f32 = 1.0;

        self.gilrs.inc();

        if ui.checkbox(&mut self.enabled, "Enabled (Start)").changed() {
            self.reset();
        };

        while let Some(event) = self.gilrs.next_event() {
            if let gilrs::EventType::ButtonPressed(Button::Start, _) = event.event {
                self.enabled = !self.enabled;
                if !self.enabled {
                    self.reset();
                }
            };
            self.active_gamepad = Some(event.id);
        }

        if let Some(gamepad) = self.active_gamepad.map(|id| self.gilrs.gamepad(id)) {
            let right = get_axis_value(gamepad, Axis::LeftStickX).unwrap_or(0.0);
            let forward = get_axis_value(gamepad, Axis::LeftStickY).unwrap_or(0.0);

            let left = -right;

            let turn_right = gamepad
                .button_data(Button::RightTrigger2)
                .map(|button| button.value())
                .unwrap_or_default();
            let turn_left = gamepad
                .button_data(Button::LeftTrigger2)
                .map(|button| button.value())
                .unwrap_or_default();
            let turn = turn_left - turn_right;

            let head_pitch = get_axis_value(gamepad, Axis::RightStickY).unwrap_or(0.0);
            let head_yaw = -get_axis_value(gamepad, Axis::RightStickX).unwrap_or(0.0);

            let injected_head_joints = HeadJoints {
                yaw: head_yaw * HEAD_YAW_SCALE,
                pitch: head_pitch * HEAD_PITCH_SCALE,
            };

            let step = Step {
                forward,
                left,
                turn,
            };

            if self.enabled {
                let now = SystemTime::now();
                if now
                    .duration_since(self.last_update)
                    .expect("Time ran backwards")
                    > UPDATE_DELAY
                {
                    self.last_update = now;
                    self.update_step(serde_json::to_value(step).unwrap());
                    self.update_look_at_angle(serde_json::to_value(injected_head_joints).unwrap());
                }
            }

            ui.vertical(|ui| {
                let label_1 = ui.label(&format!("{:#?}", step));
                let label_2 = ui.label(&format!("{:#?}", injected_head_joints));

                label_1.union(label_2)
            })
            .inner
        } else {
            ui.label("No controller found")
        }
    }
}
