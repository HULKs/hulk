use booster_sdk::client::light_control::SetLedLightColorParameter;
use color_eyre::Result;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use hardware::{LightControlInterface, SimulatorInterface};
use types::primary_state::PrimaryState;

#[derive(Deserialize, Serialize)]
pub struct LEDHandler {
    last_primary_state: Option<PrimaryState>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    primary_state: Input<PrimaryState, "primary_state">,

    hardware_interface: HardwareInterface,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {}

impl LEDHandler {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_primary_state: None,
        })
    }

    pub fn cycle(
        &mut self,
        context: CycleContext<impl SimulatorInterface + LightControlInterface>,
    ) -> Result<MainOutputs> {
        if context.hardware_interface.is_simulation()? {
            return Ok(MainOutputs {});
        }

        if self.last_primary_state == Some(*context.primary_state) {
            return Ok(MainOutputs {});
        }

        let light_control_parameter = match context.primary_state {
            PrimaryState::Safe => SetLedLightColorParameter::BLUE,
            PrimaryState::Stop => SetLedLightColorParameter::LIGHT_BLUE,
            PrimaryState::Ready => SetLedLightColorParameter::LIGHT_GREEN,
            PrimaryState::Initial => SetLedLightColorParameter::YELLOW,
            PrimaryState::Set => SetLedLightColorParameter::ORANGE,
            PrimaryState::Playing => SetLedLightColorParameter::GREEN,
            PrimaryState::Penalized => SetLedLightColorParameter::LIGHT_RED,
            PrimaryState::Finished => SetLedLightColorParameter::PURPLE,
        };

        context
            .hardware_interface
            .set_led_color(light_control_parameter)?;

        self.last_primary_state = Some(*context.primary_state);

        Ok(MainOutputs {})
    }
}

pub trait DefaultLEDColors {
    const BLUE: Self;
    const LIGHT_BLUE: Self;
    const RED: Self;
    const LIGHT_RED: Self;
    const GREEN: Self;
    const LIGHT_GREEN: Self;
    const ORANGE: Self;
    const YELLOW: Self;
    const PURPLE: Self;
}

impl DefaultLEDColors for SetLedLightColorParameter {
    const BLUE: Self = SetLedLightColorParameter { r: 0, g: 0, b: 255 };
    const RED: Self = SetLedLightColorParameter { r: 255, g: 0, b: 0 };
    const GREEN: Self = SetLedLightColorParameter { r: 0, g: 255, b: 0 };
    const LIGHT_BLUE: Self = SetLedLightColorParameter {
        r: 128,
        g: 128,
        b: 255,
    };
    const LIGHT_RED: Self = SetLedLightColorParameter {
        r: 255,
        g: 128,
        b: 128,
    };
    const LIGHT_GREEN: Self = SetLedLightColorParameter {
        r: 128,
        g: 255,
        b: 128,
    };
    const ORANGE: Self = SetLedLightColorParameter {
        r: 255,
        g: 128,
        b: 0,
    };
    const YELLOW: Self = SetLedLightColorParameter {
        r: 255,
        g: 255,
        b: 0,
    };
    const PURPLE: Self = SetLedLightColorParameter {
        r: 128,
        g: 0,
        b: 255,
    };
}
