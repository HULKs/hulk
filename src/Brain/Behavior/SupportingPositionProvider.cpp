#include "Tools/Chronometer.hpp"
#include "Tools/Math/Angle.hpp"

#include "Brain/Behavior/SupportingPositionProvider.hpp"


SupportingPositionProvider::SupportingPositionProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , fieldDimensions_(*this)
  , gameControllerState_(*this)
  , playingRoles_(*this)
  , teamBallModel_(*this)
  , worldState_(*this)
  , supportingPosition_(*this)
  , minimumAngle_(*this, "minimumAngle", [this] { minimumAngle_() *= TO_RAD; })
  , distanceToBall_(*this, "distanceToBall", [] {})
  , supporterClipGoalLineOffsetX_(*this, "supporterClipGoalLineOffsetX", [] {})
{
  minimumAngle_() *= TO_RAD;
}

void SupportingPositionProvider::cycle()
{
  Chronometer time(debug(), mount_ + ".cycleTime");

  if (gameControllerState_->gameState != GameState::PLAYING || !teamBallModel_->seen)
  {
    return;
  }


  // Find the angle of the vector from our own goal to the ball. Ideally, the supporter should
  // stand on this vector.
  const Vector2f absOwnGoalPosition = Vector2f(-fieldDimensions_->fieldLength / 2, 0);
  const Vector2f ownGoalToBall = teamBallModel_->absPosition - absOwnGoalPosition;
  const float angleOwnGoalToBall = std::atan2(ownGoalToBall.y(), ownGoalToBall.x());

  // Find the angle of the vector from the ball to our opponent's goal. In most cases, the striker
  // will stand on this vector.
  const Vector2f absOpponentsGoalPosition = Vector2f(fieldDimensions_->fieldLength / 2, 0);
  const Vector2f ballToOpponentsGoal = absOpponentsGoalPosition - teamBallModel_->absPosition;
  const float angleBallToOpponentsGoal =
      std::atan2(ballToOpponentsGoal.y(), ballToOpponentsGoal.x());

  // Compute the difference of said angles.
  const float diff = Angle::angleDiff(angleOwnGoalToBall, angleBallToOpponentsGoal);

  // The optimal angle is the angle that covers our goal while allowing the supporter to see the
  // ball. If the opponent has free kick the striker blocks our goal and the supporter should be
  // offset.
  const bool opponentHasFreeKick =
      gameControllerState_->setPlay != SetPlay::NONE && !gameControllerState_->kickingTeam;
  const bool sightBlocked = std::abs(diff) < minimumAngle_();
  float optimalAngle = angleOwnGoalToBall;
  if (opponentHasFreeKick)
  {
    optimalAngle = angleOwnGoalToBall + (worldState_->ballInLeftHalf ? 1 : -1) * minimumAngle_();
  }
  else if (sightBlocked)
  {
    optimalAngle =
        angleBallToOpponentsGoal + (worldState_->ballInLeftHalf ? 1 : -1) * minimumAngle_();
  }

  // The supporting position is a specified distance away from the ball with the optimal angle.
  // Logically, this is behind the ball and towards y = 0.
  Vector2f supportingPosition =
      teamBallModel_->absPosition -
      distanceToBall_() * Vector2f(std::cos(optimalAngle), std::sin(optimalAngle));
  // the supporting position must not be too close to our own goal
  const float supporterLineX =
      (-fieldDimensions_->fieldLength / 2) + supporterClipGoalLineOffsetX_();
  supportingPosition.x() = std::max(supportingPosition.x(), supporterLineX);
  supportingPosition_->position = supportingPosition;

  // compute orientation to face ball
  const Vector2f supportingPositionToBall =
      teamBallModel_->absPosition - supportingPosition_->position;
  const float angleSupportingPositionToBall =
      std::atan2(supportingPositionToBall.y(), supportingPositionToBall.x());
  supportingPosition_->orientation = angleSupportingPositionToBall;
  supportingPosition_->valid = true;
}
