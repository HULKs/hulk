use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum MotionRuntime {
    Booster,
    #[default]
    Hulk,
}
