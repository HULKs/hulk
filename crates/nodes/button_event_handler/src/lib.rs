use std::sync::Arc;
use std::{boxed::Box, future::Future, pin::Pin};

use color_eyre::Result;

use booster::{ButtonEventMsg, ButtonEventType};
use ros_z::{prelude::*, qos::QosDurability};
use types::{
    audio::{SOUND_REQUEST_TOPIC, SOUNDBOARD_TOPIC, Sound, Soundboard, SpeakerRequest},
    buttons::{ButtonPressType, Buttons},
};

pub fn run_boxed(ctx: Arc<Context>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(run(ctx))
}

async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("button_event_handler").build().await?;
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
    let button_event_message_sub = node
        .subscriber::<ButtonEventMsg>("inputs/button_event_message")
        .build()
        .await?;
    let buttons_pub = node
        .publisher::<Buttons<Option<ButtonPressType>>>("buttons")
        .build()
        .await?;

    let mut last_button_event_types: Buttons<Option<ButtonEventType>> = Default::default();

    loop {
        let button_event_message = button_event_message_sub.recv().await?;

        let mut buttons = Buttons::default();

        buttons[button_event_message.button] = ButtonPressType::from_button_event_types(
            &last_button_event_types[button_event_message.button],
            &button_event_message.event,
        );
        last_button_event_types[button_event_message.button] = Some(button_event_message.event);

        buttons_pub.publish(&buttons).await?;
        if let Some(soundboard) = soundboard_cache.get_latest()
            && let Some(sound) = sound_for_buttons(&soundboard, &buttons)
        {
            sound_pub
                .publish(&SpeakerRequest::PlaySound { sound })
                .await?;
        }
    }
}

fn sound_for_buttons(
    soundboard: &Soundboard,
    buttons: &Buttons<Option<ButtonPressType>>,
) -> Option<Sound> {
    (soundboard.sigh && matches!(buttons.f1, Some(ButtonPressType::Short))).then_some(Sound::Sigh)
}
