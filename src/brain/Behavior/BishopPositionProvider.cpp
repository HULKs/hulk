#include "Tools/Chronometer.hpp"

#include "BishopPositionProvider.hpp"


BishopPositionProvider::BishopPositionProvider(const ModuleManagerInterface& manager)
  : Module(manager, "BishopPositionProvider")
  , fieldDimensions_(*this)
  , gameControllerState_(*this)
  , playingRoles_(*this)
  , teamBallModel_(*this)
  , worldState_(*this)
  , bishopPosition_(*this)
{
}

void BishopPositionProvider::cycle()
{
  Chronometer time(debug(), mount_ + ".cycleTime");
  if (gameControllerState_->state != GameState::PLAYING || playingRoles_->role != PlayingRole::BISHOP || !teamBallModel_->seen)
  {
    return;
  }

  const float xPosition = worldState_->ballInOwnHalf ? fieldDimensions_->fieldLength * 0.5f - 2.f : fieldDimensions_->fieldLength * 0.5f - 1.f;
  const float yPosition = worldState_->ballInLeftHalf ? 1.f : -1.f;
  bishopPosition_->position = Vector2f(xPosition, yPosition);
  bishopPosition_->valid = true;
}
