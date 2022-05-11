use macros::SerializeHierarchy;
use spl_network::{GameControllerStateMessage, SplMessage};

#[derive(Clone, Debug, Default, SerializeHierarchy)]
pub struct MainOutputs {
    #[leaf]
    pub game_controller_state_message: Option<GameControllerStateMessage>,
    #[leaf]
    pub spl_message: Option<SplMessage>,
}

#[derive(Debug, Default, Clone, SerializeHierarchy)]
pub struct AdditionalOutputs {}

#[derive(Debug, Default, Clone)]
pub struct Database {
    pub main_outputs: MainOutputs,
    pub additional_outputs: AdditionalOutputs,
}
