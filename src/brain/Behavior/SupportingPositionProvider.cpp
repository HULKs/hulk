#include "Tools/Chronometer.hpp"

#include "SupportingPositionProvider.hpp"


SupportingPositionProvider::SupportingPositionProvider(const ModuleManagerInterface& manager)
  : Module(manager, "SupportingPositionProvider")
  , fieldDimensions_(*this)
  , gameControllerState_(*this)
  , playerConfiguration_(*this)
  , playingRoles_(*this)
  , robotPosition_(*this)
  , teamBallModel_(*this)
  , teamPlayers_(*this)
  , supportingPosition_(*this)
  , minimumDistance_(*this, "minimumDistance", [] {})
  , wasObstructing_(false)
{
}

void SupportingPositionProvider::cycle()
{
  Chronometer time(debug(), mount_ + ".cycleTime");

  if (gameControllerState_->state != GameState::PLAYING || playingRoles_->role != PlayingRole::SUPPORT_STRIKER || !teamBallModel_->seen)
  {
    return;
  }

  Vector2f relKickTarget;
  const TeamPlayer* passTarget = nullptr;
  findPassTarget(passTarget);
  if (passTarget != nullptr)
  {
    if (passTarget->playerNumber == playerConfiguration_->playerNumber)
    {
      supportingPosition_->position = robotPosition_->pose.position; // I'm the pass target; wait at current pose
      supportingPosition_->valid = true;
      wasObstructing_ = false;
      return;
    }
    else
    {
      relKickTarget = robotPosition_->fieldToRobot(passTarget->pose.position); // the striker wants to pass to another robot
    }
  }
  else
  {
    relKickTarget = robotPosition_->fieldToRobot(Vector2f(fieldDimensions_->fieldLength * 0.5f, 0.f)); // the kick target is the goal
  }

  const Vector2f relBallPosition = robotPosition_->fieldToRobot(teamBallModel_->position);
  const float ballToRobotDistance = 1.0f;
  // Do NOT remove parentheses
  const Vector2f ballToRobot = relBallPosition.normalized() * (-ballToRobotDistance);
  // desired supporting position (may obstruct striker though)
  const Vector2f relSupportingPosition =
      std::abs(relBallPosition.squaredNorm() - ballToRobotDistance * ballToRobotDistance) >= 0.1f * 0.1f ? relBallPosition + ballToRobot : Vector2f(0.f, 0.f);

  // compute the distance of the supporting pose and its projection to the line between ball and kick target
  const Vector2f ballToKickTarget = relKickTarget - relBallPosition;
  const Vector2f ballToSupportingPosition = relSupportingPosition - relBallPosition;
  const Vector2f projectedRelSupportingPosition =
      relBallPosition + ballToKickTarget * (ballToSupportingPosition.dot(ballToKickTarget)) / (ballToKickTarget.dot(ballToKickTarget));
  const float distanceToKickLineSquared = (projectedRelSupportingPosition - relSupportingPosition).squaredNorm();
  const float minimumDistance = wasObstructing_ ? minimumDistance_() + 0.2f : minimumDistance_();
  const bool tooCloseToKickLine = distanceToKickLineSquared < minimumDistance * minimumDistance;
  const bool betweenBallAndTarget = relSupportingPosition.x() > relBallPosition.x() && relSupportingPosition.x() < relKickTarget.x();
  if (tooCloseToKickLine && betweenBallAndTarget) // check if supporter position obstructs striker
  {
    // find shortest direction to move away from direct line between ball and kick target (to a position 1 m away from said line)
    const int sign = (relKickTarget.x() - relBallPosition.x()) * (relSupportingPosition.y() - relBallPosition.y()) >
                             (relSupportingPosition.x() - relBallPosition.x()) * (relKickTarget.y() - relBallPosition.y())
                         ? 1
                         : -1;
    const Vector2f newRelSupportingPosition =
        projectedRelSupportingPosition + Vector2f(-ballToKickTarget.y(), ballToKickTarget.x()) / ballToKickTarget.norm() * sign * minimumDistance_();
    supportingPosition_->position = robotPosition_->robotToField(newRelSupportingPosition);
    supportingPosition_->valid = true;
    wasObstructing_ = true;
  }
  else
  {
    supportingPosition_->position = robotPosition_->robotToField(relSupportingPosition);
    supportingPosition_->valid = true;
    wasObstructing_ = false;
  }
}

void SupportingPositionProvider::findPassTarget(const TeamPlayer*& passTarget)
{
  const TeamPlayer* striker = nullptr;
  for (auto& teamPlayer : teamPlayers_->players)
  {
    if (teamPlayer.penalized)
    {
      continue;
    }
    else if (teamPlayer.currentlyPerfomingRole == PlayingRole::STRIKER)
    {
      striker = &teamPlayer;
      break;
    }
  }
  if (striker != nullptr)
  {
    for (auto& teamPlayer : teamPlayers_->players)
    {
      if (teamPlayer.penalized)
      {
        continue;
      }
      else if (striker->currentPassTarget == static_cast<int>(teamPlayer.playerNumber))
      {
        passTarget = &teamPlayer;
        break;
      }
    }
  }
}
