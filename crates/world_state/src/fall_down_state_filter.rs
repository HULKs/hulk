use color_eyre::Result;
use serde::{Deserialize, Serialize};

use booster::FallDownState;
use context_attribute::context;
use framework::{MainOutput, PerceptionInput};

#[derive(Deserialize, Serialize)]
pub struct FallDownStateFilter {
    last_fall_down_state: Option<FallDownState>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    fall_down_state: PerceptionInput<Option<FallDownState>, "FallDownState", "fall_down_state?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub fall_down_state: MainOutput<Option<FallDownState>>,
}

impl FallDownStateFilter {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {
            last_fall_down_state: None,
        })
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let fall_down_state = context
            .fall_down_state
            .persistent
            .into_iter()
            .chain(context.fall_down_state.temporary)
            .flat_map(|(_time, fall_down_states)| fall_down_states)
            .last()
            .flatten()
            .copied()
            .map_or(self.last_fall_down_state, |fall_down_state| {
                Some(fall_down_state)
            });

        if fall_down_state.is_some() {
            self.last_fall_down_state = fall_down_state;
        }

        Ok(MainOutputs {
            fall_down_state: fall_down_state.into(),
        })
    }
}
