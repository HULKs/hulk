use std::sync::Arc;

use color_eyre::Result;

use booster::{ButtonEventMsg, ButtonEventType};
use ros_z::prelude::*;
use types::buttons::{ButtonPressType, Buttons};

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("button_event_handler").build().await?;
    let button_event_message_sub = node
        .subscriber::<ButtonEventMsg>("inputs/button_event_message")?
        .build()
        .await?;
    let buttons_pub = node
        .publisher::<Buttons<Option<ButtonPressType>>>("buttons")?
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
    }
}
