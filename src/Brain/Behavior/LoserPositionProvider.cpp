#include "Brain/Behavior/LoserPositionProvider.hpp"
#include "Tools/Chronometer.hpp"


LoserPositionProvider::LoserPositionProvider(const ModuleManagerInterface& manager)
  : Module{manager}
  , teamBallModel_{*this}
  , loserPosition_{*this}
{
}

void LoserPositionProvider::cycle()
{
  Chronometer time(debug(), mount_ + ".cycle_time");

  if (teamBallModel_->ballType != TeamBallModel::BallType::NONE)
  {
    lastKnownTeamBallPosition_ = teamBallModel_->absPosition;
  }

  // Loser should always go backwards
  loserPosition_->pose = Pose{lastKnownTeamBallPosition_ - Vector2f{0.5f, 0.f}, 0.f};
  loserPosition_->valid = true;
}
