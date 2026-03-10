use path_serde::{PathDeserialize, PathIntrospect, PathSerialize};
use serde::{Deserialize, Serialize};

use crate::field_dimensions::Side;

#[derive(
    Debug,
    Clone,
    Copy,
    PathSerialize,
    PathDeserialize,
    PathIntrospect,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
)]
pub enum Action {
    DefendGoal,
    DefendKickOff,
    DefendLeft,
    DefendOpponentCornerKick { side: Side },
    DefendPenaltyKick,
    Dribble,
    Finish,
    Initial,
    LookAround,
    Penalize,
    RemoteControl,
    Safe,
    StandDuringPenaltyKick,
    Stop,
    StandUp,
    VisualKick,
    WalkToBall,
    WalkToKickOff,
    WalkToPenaltyKick,
}
