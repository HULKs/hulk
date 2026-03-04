use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, Serialize, Deserialize, PathSerialize, PathDeserialize, PathIntrospect, Debug,
)]
pub enum Buttons {
    IsStandLongPressed,
    IsStandOrF1Pressed,
    IsStandLongPressedDuringSafePose,
}
