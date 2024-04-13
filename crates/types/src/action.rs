use serde::{Deserialize, Serialize};
use serialize_hierarchy::SerializeHierarchy;

#[derive(Debug, Clone, Copy, SerializeHierarchy, Serialize, Deserialize)]
pub enum Action {
    Calibrate,
    DefendGoal,
    DefendKickOff,
    DefendLeft,
    DefendPenaltyKick,
    DefendRight,
    Dribble,
    FallSafely,
    Initial,
    InterceptBall,
    Jump,
    LookAround,
    NoGroundContact,
    Penalize,
    PrepareJump,
    Search,
    SearchForLostBall,
    SitDown,
    Stand,
    StandUp,
    SupportLeft,
    SupportRight,
    SupportStriker,
    Unstiff,
    WalkToKickOff,
    WalkToPenaltyKick,
}
