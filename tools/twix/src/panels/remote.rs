use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, SystemTime},
};

use communication::messages::TextOrBinary;
use eframe::egui::Widget;
use gilrs::{Axis, Button, Gamepad, GamepadId, Gilrs};
use serde_json::{json, Value};
use types::step::Step;

use crate::{nao::Nao, panel::Panel, value_buffer::BufferHandle};

pub struct RemotePanel {
    nao: Arc<Nao>,
    enabled: Arc<AtomicBool>,
    latest_step: BufferHandle<Step>,
}
impl Panel for RemotePanel {
    const NAME: &'static str = "Remote";

    fn new(nao: Arc<Nao>, _value: Option<&Value>) -> Self {
        let enabled = Arc::new(AtomicBool::new(false));
        let latest_step = nao.subscribe_value("parameters.remote_control_parameters.walk");

        let nao_clone = nao.clone();
        let enabled_clone = enabled.clone();

        thread::spawn(move || {
            let mut gilrs = match Gilrs::new() {
                Ok(g) => g,
                Err(e) => {
                    eprintln!("failed to init gilrs in bg thread: {e}");
                    return;
                }
            };
            const UPDATE_DELAY: Duration = Duration::from_millis(100);
            const SLEEP_DELAY: Duration = Duration::from_millis(10);

            let mut active_gamepad: Option<GamepadId> = None;
            let mut last_update = SystemTime::now()
                .checked_sub(UPDATE_DELAY)
                .unwrap_or(SystemTime::now());

            loop { // TODO stop thread
                gilrs.inc();
                while let Some(event) = gilrs.next_event() {
                    active_gamepad = Some(event.id);
                }
                if let Some(gamepad) = active_gamepad.map(|id| gilrs.gamepad(id)) {
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

                    let step = Step {
                        forward,
                        left,
                        turn,
                    };

                    if gamepad
                        .button_data(Button::Start)
                        .map(|button| button.is_pressed())
                        .unwrap_or(false)
                    {
                        enabled_clone.store(!enabled_clone.load(Ordering::Relaxed), Ordering::Relaxed);
                    } //TODO not working

                    if enabled_clone.load(Ordering::Relaxed) {
                        let now = SystemTime::now();
                        if now.duration_since(last_update).expect("Time ran backwards")
                            > UPDATE_DELAY
                        {
                            last_update = now;
                            nao_clone.write(
                                "parameters.remote_control_parameters.walk",
                                TextOrBinary::Text(serde_json::to_value(step).unwrap()),
                            )
                        }
                    }
                }
                thread::sleep(SLEEP_DELAY);
            }
        });
        Self {
            nao,
            enabled,
            latest_step,
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
        self.update_step(serde_json::to_value(Step::<f32>::default()).unwrap());
    }

    fn update_step(&self, step: Value) {
        self.nao.write(
            "parameters.remote_control_parameters.walk",
            TextOrBinary::Text(step),
        )
    }
}

impl Widget for &mut RemotePanel {
    fn ui(self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        let mut enabled = self.enabled.load(Ordering::Relaxed);
        if ui.checkbox(&mut enabled, "Enabled (Start)").changed() {
            self.enabled.store(enabled, Ordering::Relaxed);
            if !enabled {
                self.reset();
            }
        };

        let step = match self.latest_step.get_last_value() {
            Ok(Some(step)) => step,
            _ => Step::default(),
        };

        ui.label(format!("{step:#?}"))
    }
}
