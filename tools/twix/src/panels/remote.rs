use std::sync::Arc;

use eframe::egui::Widget;
use gilrs::{Axis, Event, GamepadId, Gilrs};
use serde_json::{json, Value};
use types::Step;

use crate::{nao::Nao, panel::Panel};

pub struct RemotePanel {
    nao: Arc<Nao>,
    gilrs: Gilrs,
    active_gamepad: Option<GamepadId>,
}

impl Panel for RemotePanel {
    const NAME: &'static str = "Remote";

    fn new(nao: Arc<Nao>, _value: Option<&Value>) -> Self {
        let gilrs = Gilrs::new().unwrap();
        let active_gamepad = None;

        Self {
            nao,
            gilrs,
            active_gamepad,
        }
    }

    fn save(&self) -> Value {
        json!({})
    }
}

impl Widget for &mut RemotePanel {
    fn ui(self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        while let Some(Event { id, .. }) = self.gilrs.next_event() {
            self.active_gamepad = Some(id);
        }

        if let Some(gamepad) = self.active_gamepad.map(|id| self.gilrs.gamepad(id)) {
            let left = match gamepad.axis_data(Axis::LeftStickX) {
                Some(data) => -data.value(), // inverted because left is negative on the joystick
                _ => 0.0,
            };
            let forward = match gamepad.axis_data(Axis::LeftStickY) {
                Some(data) => data.value(),
                _ => 0.0,
            };
            let turn = match gamepad.axis_data(Axis::RightStickX) {
                Some(data) => data.value(),
                _ => 0.0,
            };

            let step = Step {
                forward,
                left,
                turn,
            };

            self.nao.update_parameter_value(
                "control.step_planner.injected_step",
                serde_json::to_value(step).unwrap(),
            );
            ui.label(&format!("{:#?}", step))
        } else {
            ui.label("No controller found")
        }
    }
}
