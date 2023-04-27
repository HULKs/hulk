use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Debug, Clone, Copy, SerializeHierarchy, Serialize, Deserialize)]
pub enum Action {
    Unstiff,
    SitDown,
    Penalize,
    Initial,
    FallSafely,
    StandUp,
    Stand,

    LookAround,
    Dribble,
    DefendGoal,
    DefendKickOff,
    DefendLeft,
    DefendRight,
    DefendPenaltyKick,
    Jump,
    PrepareJump,
    SupportLeft,
    SupportRight,
    SupportStriker,
    Search,
    SearchForLostBall,
    WalkToKickOff,
    WalkToPenaltyKick,
}
