#include "Tools/Chronometer.hpp"
#include "Tools/Math/Angle.hpp"

#include "BishopPositionProvider.hpp"


BishopPositionProvider::BishopPositionProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , minimumAngle_(*this, "minimumAngle", [this] { minimumAngle_() *= TO_RAD; })
  , distanceToBall_(*this, "distanceToBall", [] {})
  , allowAggressiveBishop_(*this, "allowAggressiveBishop", [] {})
  , fieldDimensions_(*this)
  , gameControllerState_(*this)
  , playingRoles_(*this)
  , robotPosition_(*this)
  , supportingPosition_(*this)
  , teamBallModel_(*this)
  , teamPlayers_(*this)
  , worldState_(*this)
  , bishopPosition_(*this)
  , aggressiveBishopLineX_(-fieldDimensions_->fieldLength / 2 + 3.f)
{
  minimumAngle_() *= TO_RAD;
}

void BishopPositionProvider::cycle()
{
  Chronometer time(debug(), mount_ + ".cycleTime");
  if (gameControllerState_->gameState != GameState::PLAYING ||
      playingRoles_->role != PlayingRole::BISHOP || !teamBallModel_->seen)
  {
    return;
  }

  if (allowAggressiveBishop_() && beAggressive())
  {
    // the bishop position
    const float xPosition = fieldDimensions_->fieldLength * 0.5f - 1.f;
    const float yPosition = worldState_->ballInLeftHalf ? -1.f : 1.f;
    const Vector2f absBishopPosition = Vector2f(xPosition, yPosition);

    // the bishop orientation
    const Vector2f absBallPosition = teamBallModel_->position;
    const Vector2f bishopToBall = absBallPosition - absBishopPosition;
    const Vector2f absGoalPosition = Vector2f(fieldDimensions_->fieldLength / 2.f, 0.f);
    const Vector2f bishopToGoal = absGoalPosition - absBishopPosition;
    const Vector2f orientationVector = bishopToBall.normalized() + bishopToGoal.normalized();
    const float absBishopAngle = std::atan2(orientationVector.y(), orientationVector.x());

    bishopPosition_->position = absBishopPosition;
    bishopPosition_->orientation = absBishopAngle;
    bishopPosition_->valid = true;
  }
  else
  {
    // compute angle of vector from supporting position to ball
    const Vector2f absBallPosition = teamBallModel_->position;
    const Vector2f absSupportingPosition = supportingPosition_->valid
                                               ? supportingPosition_->position
                                               : Vector2f(-fieldDimensions_->fieldLength / 2, 0);
    const Vector2f supportingPositionToBall = absBallPosition - absSupportingPosition;
    const float angleSupportingPositionToBall =
        std::atan2(supportingPositionToBall.y(), supportingPositionToBall.x());

    // compute desired angle of vector from bishop position to ball
    const float angleBishopPositionToBall =
        angleSupportingPositionToBall + (worldState_->ballInLeftHalf ? 1 : -1) * minimumAngle_();

    // compute bishop position from desired angle
    Vector2f absBishopPosition =
        absBallPosition - distanceToBall_() * Vector2f(std::cos(angleBishopPositionToBall),
                                                       std::sin(angleBishopPositionToBall));
    // the bishop position must not be too close to our own goal
    absBishopPosition.x() = std::max(absBishopPosition.x(), aggressiveBishopLineX_);

    // compute bishop orientation
    const Vector2f bishopPositionToBall = absBallPosition - absBishopPosition;
    const float absBishopAngle = std::atan2(bishopPositionToBall.y(), bishopPositionToBall.x());

    bishopPosition_->position = absBishopPosition;
    bishopPosition_->orientation = absBishopAngle;
    bishopPosition_->valid = true;
  }
}

bool BishopPositionProvider::beAggressive() const
{
  const bool enemyHasFreeKick =
      gameControllerState_->setPlay != SetPlay::NONE && !gameControllerState_->kickingTeam;
  return !enemyHasFreeKick && !worldState_->ballInOwnHalf;
}
