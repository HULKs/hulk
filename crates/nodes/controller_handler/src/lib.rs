use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

use color_eyre::Result;
use gilrs::{Gilrs, ev::AxisOrBtn};
use ros_z::{prelude::*, qos::QosDurability};
use tokio::time::{MissedTickBehavior, interval};
use tracing::warn;
use types::{
    audio::{SOUND_REQUEST_TOPIC, SOUNDBOARD_TOPIC, Sound, Soundboard, SpeakerRequest},
    controller_input::{Axis, Button, ControllerAxis, ControllerButton, ControllerInput},
};

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("controller_handler").build().await?;

    let soundboard_cache = node
        .subscriber::<Soundboard>(SOUNDBOARD_TOPIC)
        .qos(QosProfile {
            durability: QosDurability::TransientLocal,
            ..Default::default()
        })
        .cache(1)
        .build()
        .await?;
    let sound_pub = node
        .publisher::<SpeakerRequest>(SOUND_REQUEST_TOPIC)
        .build()
        .await?;
    let controller_input_pub = node
        .publisher::<ControllerInput>("inputs/controller_input")
        .build()
        .await?;

    let mut gilrs = match Gilrs::new() {
        Ok(gilrs) => gilrs,
        Err(error) => {
            warn!(%error, "failed to initialize controller handler");
            return Ok(());
        }
    };
    let mut ticker = interval(Duration::from_millis(20));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut emergency_button_was_pressed = false;

    loop {
        ticker.tick().await;
        let controller_input = read_controller_input(&mut gilrs);
        controller_input_pub.publish(&controller_input).await?;
        let sound_enabled = soundboard_cache
            .get_latest()
            .is_some_and(|soundboard| soundboard.is_enabled(Sound::MetalBar));
        if let Some(sound) = sound_for_controller_input(
            sound_enabled,
            &controller_input,
            &mut emergency_button_was_pressed,
        ) {
            sound_pub
                .publish(&SpeakerRequest::PlaySound { sound })
                .await?;
        }
    }
}

fn sound_for_controller_input(
    enabled: bool,
    input: &ControllerInput,
    was_pressed: &mut bool,
) -> Option<Sound> {
    // West is controller X, the same button used for emergency damping.
    let pressed = input.connected && input.is_pressed(Button::West);
    let play_sound = enabled && pressed && !*was_pressed;
    // Track the edge even when disabled, so enabling while held does not replay it.
    *was_pressed = pressed;
    play_sound.then_some(Sound::MetalBar)
}

fn read_controller_input(gilrs: &mut Gilrs) -> ControllerInput {
    while gilrs.next_event().is_some() {}

    let Some((_, gamepad)) = gilrs.gamepads().next() else {
        return ControllerInput::default();
    };

    let mut input = ControllerInput {
        connected: true,
        device_name: gamepad.name().to_owned(),
        axes: Vec::new(),
        buttons: Vec::new(),
    };

    for (code, data) in gamepad.state().axes() {
        if let Some(AxisOrBtn::Axis(axis)) = gamepad.axis_or_btn_name(code) {
            input.axes.push(ControllerAxis {
                name: match axis {
                    gilrs::Axis::LeftStickX => Axis::LeftStickX,
                    gilrs::Axis::LeftStickY => Axis::LeftStickY,
                    gilrs::Axis::LeftZ => Axis::LeftZ,
                    gilrs::Axis::RightStickX => Axis::RightStickX,
                    gilrs::Axis::RightStickY => Axis::RightStickY,
                    gilrs::Axis::RightZ => Axis::RightZ,
                    gilrs::Axis::DPadX => Axis::DPadX,
                    gilrs::Axis::DPadY => Axis::DPadY,
                    gilrs::Axis::Unknown => Axis::Unknown,
                },
                value: data.value(),
            });
        }
    }

    for (code, data) in gamepad.state().buttons() {
        if let Some(AxisOrBtn::Btn(button)) = gamepad.axis_or_btn_name(code) {
            input.buttons.push(ControllerButton {
                name: match button {
                    gilrs::Button::South => Button::South,
                    gilrs::Button::East => Button::East,
                    gilrs::Button::North => Button::North,
                    gilrs::Button::West => Button::West,
                    gilrs::Button::C => Button::C,
                    gilrs::Button::Z => Button::Z,
                    gilrs::Button::LeftTrigger => Button::LeftTrigger,
                    gilrs::Button::LeftTrigger2 => Button::LeftTrigger2,
                    gilrs::Button::RightTrigger => Button::RightTrigger,
                    gilrs::Button::RightTrigger2 => Button::RightTrigger2,
                    gilrs::Button::Select => Button::Select,
                    gilrs::Button::Start => Button::Start,
                    gilrs::Button::Mode => Button::Mode,
                    gilrs::Button::LeftThumb => Button::LeftThumb,
                    gilrs::Button::RightThumb => Button::RightThumb,
                    gilrs::Button::DPadUp => Button::DPadUp,
                    gilrs::Button::DPadDown => Button::DPadDown,
                    gilrs::Button::DPadLeft => Button::DPadLeft,
                    gilrs::Button::DPadRight => Button::DPadRight,
                    gilrs::Button::Unknown => Button::Unknown,
                },
                pressed: data.is_pressed(),
                value: data.value(),
            });
        }
    }

    input
}
