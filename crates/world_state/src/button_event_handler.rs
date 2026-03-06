use std::time::SystemTime;

use color_eyre::Result;
use hardware::SimulatorInterface;
use serde::{Deserialize, Serialize};

use booster::{ButtonEventMsg, ButtonEventType};
use context_attribute::context;
use framework::{MainOutput, PerceptionInput};
use types::{
    buttons::{ButtonPressType, Buttons},
    cycle_time::CycleTime,
};

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

    cycle_time: Input<CycleTime, "cycle_time">,
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

        let button_event_messages: Vec<ButtonEventMsg> = context
            .maybe_button_event
            .persistent
            .into_iter()
            .chain(context.maybe_button_event.temporary)
            .filter(|(time, _)| *time > self.most_recently_processed_button_event_message_time)
            .flat_map(|(_, button_event_messages)| button_event_messages)
            .flatten()
            .cloned()
            .collect();

        self.most_recently_processed_button_event_message_time = context.cycle_time.start_time;

        let mut buttons = Buttons::default();

        for button_event_message in button_event_messages {
            buttons[button_event_message.button] = ButtonPressType::from_button_event_types(
                &self.last_button_event_types[button_event_message.button],
                &button_event_message.event,
            );
            self.last_button_event_types[button_event_message.button] =
                Some(button_event_message.event);
        }

        Ok(MainOutputs {
            buttons: buttons.into(),
        })
    }
}
