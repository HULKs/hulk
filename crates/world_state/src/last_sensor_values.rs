// Zero-order hold for some perception inputs needed throughout the world_state crate.
// Should become obsolete with the new framework :)

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use booster::{FallDownState, ImuState};
use context_attribute::context;
use framework::{MainOutput, PerceptionInput};

#[derive(Deserialize, Serialize, Default)]
pub struct LastSensorValues {
    pub last_imu_state: ImuState,
    pub last_fall_down_state: Option<FallDownState>,
}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    imu_state: PerceptionInput<ImuState, "Motion", "imu_state">,
    fall_down_state: PerceptionInput<Option<FallDownState>, "FallDownState", "fall_down_state?">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub imu_state: MainOutput<ImuState>,
    pub fall_down_state: MainOutput<Option<FallDownState>>,
}

impl LastSensorValues {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self::default())
    }

    pub fn cycle(&mut self, context: CycleContext) -> Result<MainOutputs> {
        let imu_state = context
            .imu_state
            .persistent
            .into_iter()
            .chain(context.imu_state.temporary)
            .flat_map(|(_time, info)| info)
            .last()
            .cloned()
            .unwrap_or(self.last_imu_state);

        self.last_imu_state = imu_state;

        let fall_down_state = context
            .fall_down_state
            .persistent
            .into_iter()
            .chain(context.fall_down_state.temporary)
            .flat_map(|(_time, info)| info)
            .last()
            .map(|x| x.cloned())
            .unwrap_or(self.last_fall_down_state.clone());

        self.last_fall_down_state = fall_down_state.clone();

        Ok(MainOutputs {
            imu_state: imu_state.into(),
            fall_down_state: fall_down_state.into(),
        })
    }
}
