#include "Tools/Chronometer.hpp"
#include "Tools/Math/Angle.hpp"
#include "Tools/Math/Range.hpp"

#include "BishopPositionProvider.hpp"


BishopPositionProvider::BishopPositionProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , minimumAngle_(*this, "minimumAngle", [this] { minimumAngle_() *= TO_RAD; })
  , distanceToBall_(*this, "distanceToBall", [] {})
  , allowAggressiveBishop_(*this, "allowAggressiveBishop", [] {})
  , defaultPositionOffset_(*this, "defaultPositionOffset", [] {})
  , cornerKickOffset_(*this, "cornerKickOffset", [] {})
  , goalhangerOffset_(*this, "goalhangerOffset", [] {})
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

  // update side
  determineLeftOrRight();

  const Vector2f absBallPosition = teamBallModel_->position;
  if (allowAggressiveBishop_())
  {
    // default position (pass target for cleared balls, all other free kicks and kick-ins)
    Vector2f absBishopPosition(defaultPositionOffset_().x(),
                               static_cast<int>(side_) * defaultPositionOffset_().y());
    const bool kickingTeam =
        gameControllerState_->setPlay != SetPlay::NONE && gameControllerState_->kickingTeam;
    const bool cornerKick = worldState_->ballInCorner && !worldState_->ballInOwnHalf;
    const bool goalhanger = !worldState_->ballInOwnHalf;
    if (kickingTeam)
    {
      if (cornerKick)
      {
        // corner kick position (this includes kick-ins and fouls in the corner)
        absBishopPosition.x() = fieldDimensions_->fieldLength / 2.0f + cornerKickOffset_().x();
        absBishopPosition.y() = static_cast<int>(side_) * cornerKickOffset_().y();
      }
      else if (goalhanger)
      {
        // hang around goal if free kick or kick-in on opponents half to follow up after striker
        // kicks
        absBishopPosition.x() = fieldDimensions_->fieldLength / 2.0f + goalhangerOffset_().x();
        absBishopPosition.y() = static_cast<int>(side_) * goalhangerOffset_().y();
      }
    }

    // move bishop position away from ball if too close
    const Vector2f ballToBishop(absBishopPosition - teamBallModel_->position);
    const float distanceToBall = ballToBishop.norm();
    if (distanceToBall < distanceToBall_())
    {
      absBishopPosition += (distanceToBall_() - distanceToBall) * ballToBishop.normalized();
    }

    // compute orientation that is tradeoff between facing ball and facing opponent's goal
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

void BishopPositionProvider::determineLeftOrRight()
{
  // only change side of ball in own half to not obstruct the striker
  if (worldState_->ballInOwnHalf)
  {
    if (worldState_->ballInLeftHalf)
    {
      side_ = BishopPositionProvider::Side::RIGHT;
    }
    else
    {
      side_ = BishopPositionProvider::Side::LEFT;
    }
  }
}
