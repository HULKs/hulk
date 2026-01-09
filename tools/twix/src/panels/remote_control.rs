use crate::{
    nao::Nao,
    panel::{Panel, PanelCreationContext},
    value_buffer::BufferHandle,
};
use communication::messages::TextOrBinary;
use eframe::egui::Widget;
use gilrs::{Axis, Button, Gamepad, Gilrs};
use serde_json::{json, Value};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, SystemTime},
};
use tokio::sync::watch::{channel, Receiver};
use types::step::Step;

pub struct RemotePanel {
    nao: Arc<Nao>,
    enabled: Arc<AtomicBool>,
    latest_step: BufferHandle<Step>,
    bg_running: Arc<AtomicBool>,
    bg_handle: Option<JoinHandle<()>>,
    receiver: Receiver<Step>,
}

impl<'a> Panel<'a> for RemotePanel {
    const NAME: &'static str = "Remote";

    fn new(context: PanelCreationContext) -> Self {
        let nao = context.nao.clone();
        let (sender, receiver) = channel(Step::<f32>::default());

        let enabled = Arc::new(AtomicBool::new(false));
        let latest_step = nao.subscribe_value("parameters.remote_control_parameters.walk");
        let bg_running = Arc::new(AtomicBool::new(true));

        let nao_clone = nao.clone();
        let enabled_clone = enabled.clone();
        let bg_running_clone = bg_running.clone();
        let egui_context_clone = context.egui_context.clone();

        let handle = thread::spawn(move || {
            let mut gilrs = match Gilrs::new() {
                Ok(gilrs) => gilrs,
                Err(error) => {
                    eprintln!("failed to init gilrs in bg thread: {error}");
                    return;
                }
            };
            const UPDATE_DELAY: Duration = Duration::from_millis(100);
            let mut last_update = SystemTime::now()
                .checked_sub(UPDATE_DELAY)
                .unwrap_or(SystemTime::now());

            let mut start_was_pressed = false;

            let mut last_gamepad_id = None;

            while bg_running_clone.load(Ordering::Relaxed) {
                let event = gilrs.next_event_blocking(Some(Duration::from_secs(1)));
                if let Some(event) = &event {
                    gilrs.inc();
                    last_gamepad_id = Some(event.id);
                }

                if gilrs.gamepads().next().is_none() {
                    let _ = sender.send(Step::default());
                    if enabled_clone.load(Ordering::Relaxed) {
                        reset(&nao_clone);
                    }
                    continue;
                }

                let active_gamepad = event
                    .map(|e| e.id)
                    .or(last_gamepad_id)
                    .filter(|&id| gilrs.gamepads().any(|(g_id, _)| g_id == id))
                    .or_else(|| gilrs.gamepads().next().map(|(id, _)| id));

                if let Some(gamepad) = active_gamepad.map(|id| gilrs.gamepad(id)) {
                    last_gamepad_id = Some(gamepad.id());
                    egui_context_clone.request_repaint();
                    let (forward, right) = apply_dead_zone(
                        get_axis_value(gamepad, Axis::LeftStickX).unwrap_or(0.0),
                        get_axis_value(gamepad, Axis::LeftStickY).unwrap_or(0.0),
                    );

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

                    let start_pressed = gamepad
                        .button_data(Button::Start)
                        .map(|button| button.is_pressed())
                        .unwrap_or(false);

                    if start_pressed && !start_was_pressed {
                        let new_state = !enabled_clone.load(Ordering::Relaxed);
                        enabled_clone.store(new_state, Ordering::Relaxed);

                        if !new_state {
                            reset(&nao_clone);
                        }
                    }
                    start_was_pressed = start_pressed;

                    if enabled_clone.load(Ordering::Relaxed) {
                        let now = SystemTime::now();
                        if now.duration_since(last_update).expect("Time ran backwards")
                            > UPDATE_DELAY
                        {
                            last_update = now;
                            update_step(&nao_clone, step);
                        }
                    }
                    let _ = sender.send(step);
                }
            }
        });

        Self {
            nao,
            enabled,
            latest_step,
            bg_running,
            bg_handle: Some(handle),
            receiver,
        }
    }

    fn save(&self) -> Value {
        json!({})
    }
}

fn apply_dead_zone(x: f32, y: f32) -> (f32, f32) {
    const DEAD_ZONE: f32 = 0.15;
    if (x * x + y * y).sqrt() < DEAD_ZONE {
        return (0.0, 0.0);
    }
    (y, x)
}

fn get_axis_value(gamepad: Gamepad, axis: Axis) -> Option<f32> {
    Some(gamepad.axis_data(axis)?.value())
}

fn reset(nao: &Arc<Nao>) {
    update_step(nao, Step::<f32>::default());
}

fn update_step(nao: &Arc<Nao>, step: Step) {
    nao.write(
        "parameters.remote_control_parameters.walk",
        TextOrBinary::Text(serde_json::to_value(step).unwrap()),
    );
}

impl Drop for RemotePanel {
    fn drop(&mut self) {
        self.bg_running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.bg_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Widget for &mut RemotePanel {
    fn ui(self, ui: &mut eframe::egui::Ui) -> eframe::egui::Response {
        let mut enabled = self.enabled.load(Ordering::Relaxed);
        if ui.checkbox(&mut enabled, "Enabled (Start)").changed() {
            self.enabled.store(enabled, Ordering::Relaxed);
            if !enabled {
                reset(&self.nao);
            }
        };
        ui.separator();
        ui.strong("Controller:");
        let controller_step = *self.receiver.borrow();
        ui.label(format!("{controller_step:#?}"));
        ui.add_space(ui.spacing().item_spacing.y);
        ui.strong("Robot:");

        match self.latest_step.get_last_value() {
            Ok(Some(step)) => ui.label(format!("{step:#?}")),
            _ => ui.label("No data"),
        }
    }
}
