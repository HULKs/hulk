use std::{sync::Arc, time::Duration};

use color_eyre::Result;

use booster::{ButtonEventMsg, ButtonEventType};
use ros_z::{prelude::*, time::Time};
use ros_z_streams::{CreateAnnouncingPublisher, CreateFutureMapBuilder};
use types::buttons::{ButtonPressType, Buttons};

pub async fn run(ctx: Arc<Context>) -> Result<()> {
    let node = ctx.create_node("button_event_handler").build().await?;
    let mut button_event_future_map = node
        .create_future_map_builder()
        .create_future_subscriber::<ButtonEventMsg>(
            "inputs/button_event_message",
            Duration::from_millis(1),
        )
        .await?
        .build();
    let buttons_pub = node
        .announcing_publisher::<Buttons<Option<ButtonPressType>>>("buttons")
        .await?;

    let mut last_button_event_types: Buttons<Option<ButtonEventType>> = Default::default();
    let mut most_recently_processed_button_event_message_time: Time = Time::zero();

    loop {
        let button_event_message_item = button_event_future_map.recv().await?;

        let all_button_event_messages: Vec<(&Time, &(Option<ButtonEventMsg>,))> =
            button_event_message_item
                .persistent
                .iter()
                .chain(button_event_message_item.temporary)
                .filter(|(time, _)| *time > &most_recently_processed_button_event_message_time)
                .collect();

        let mut buttons = Buttons::default();

        for (time, button_event_message) in all_button_event_messages {
            let (Some(button_event_message),) = button_event_message else {
                continue;
            };
            most_recently_processed_button_event_message_time = *time;

            buttons[button_event_message.button] = ButtonPressType::from_button_event_types(
                &last_button_event_types[button_event_message.button],
                &button_event_message.event,
            );
            last_button_event_types[button_event_message.button] = Some(button_event_message.event);
        }
        let pending_announcement = buttons_pub
            .announce(most_recently_processed_button_event_message_time)
            .await?;

        pending_announcement.publish(&buttons).await?;
    }
}
