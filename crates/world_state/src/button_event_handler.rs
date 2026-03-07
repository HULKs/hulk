use std::time::SystemTime;

use color_eyre::Result;
use hardware::SimulatorInterface;
use serde::{Deserialize, Serialize};

use booster::{ButtonEventMsg, ButtonEventType};
use context_attribute::context;
use framework::{MainOutput, PerceptionInput};
use types::buttons::{ButtonPressType, Buttons};

#[derive(Deserialize, Serialize)]
pub struct ButtonEventHandler {
    pub last_button_event_types: Buttons<Option<ButtonEventType>>,
    pub most_recently_processed_button_event_message_time: SystemTime,
}

#[context]
pub struct CreationContext {}
#[context]
pub struct CycleContext {
    maybe_button_event: PerceptionInput<Option<ButtonEventMsg>, "ButtonEvent", "button_event?">,

    hardware_interface: HardwareInterface,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub buttons: MainOutput<Buttons<Option<ButtonPressType>>>,
}

impl ButtonEventHandler {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_button_event_types: Default::default(),
            most_recently_processed_button_event_message_time: SystemTime::UNIX_EPOCH,
        })
    }

    pub fn cycle(&mut self, context: CycleContext<impl SimulatorInterface>) -> Result<MainOutputs> {
        if context.hardware_interface.is_simulation()? {
            return Ok(MainOutputs {
                buttons: Buttons::default().into(),
            });
        }

        let all_button_event_messages: Vec<(SystemTime, Vec<Option<&ButtonEventMsg>>)> = context
            .maybe_button_event
            .persistent
            .into_iter()
            .chain(context.maybe_button_event.temporary)
            .filter(|(time, _)| *time > self.most_recently_processed_button_event_message_time)
            .collect();

        let mut buttons = Buttons::default();

        for (time, button_event_messages) in all_button_event_messages {
            self.most_recently_processed_button_event_message_time = time;

            button_event_messages.into_iter().flatten().for_each(
                |ButtonEventMsg { button, event }| {
                    buttons[*button] = ButtonPressType::from_button_event_types(
                        &self.last_button_event_types[*button],
                        event,
                    );
                    self.last_button_event_types[*button] = Some(*event);
                },
            );
        }

        Ok(MainOutputs {
            buttons: buttons.into(),
        })
    }
}
