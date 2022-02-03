#include "Tools/Chronometer.hpp"
#include "Tools/PermissionManagement.hpp"

#include "Brain/Behavior/ReplacementKeeperActionProvider.hpp"


ReplacementKeeperActionProvider::ReplacementKeeperActionProvider(
    const ModuleManagerInterface& manager)
  : Module(manager)
  , gameControllerState_(*this)
  , robotPosition_(*this)
  , teamBallModel_(*this)
  , keeperAction_(*this)
  , worldState_(*this)
  , replacementKeeperAction_(*this)
{
}

void ReplacementKeeperActionProvider::cycle()
{
  Chronometer time(debug(), mount_ + ".cycleTime");

  // find the best action that the replacment keeper is permitted to perform, assuming the actions
  // are sorted
  for (const auto& action : keeperAction_->actions)
  {
    if (action.valid)
    {
      if (PermissionManagement::checkPermission(
              static_cast<unsigned int>(action.type),
              static_cast<unsigned int>(replacementKeeperAction_->permission)))
      {
        replacementKeeperAction_->action = action;
        break;
      }
    }
  }

  if (replacementKeeperAction_->action.valid)
  {
    considerSetPlay();
  }
}

void ReplacementKeeperActionProvider::considerSetPlay()
{
  const bool enemyHasFreeKick =
      gameControllerState_->setPlay != SetPlay::NONE && !gameControllerState_->kickingTeam;
  // If there is no free kick there is nothing to do
  if (!enemyHasFreeKick)
  {
    return;
  }

  // If the team ball was not found, we are not able to go away from it
  if (!teamBallModel_->found)
  {
    return;
  }

  if (worldState_->ballInOwnHalf && gameControllerState_->setPlay == SetPlay::GOAL_KICK &&
      !gameControllerState_->kickingTeam)
  {
    // ref made a mistake: The ball cannot be in our own half without us being the kicking team
    // during a GOALFreeKick
    return;
  }

  // vector to the ball (relative to the robot)
  const Vector2f ballToRobot = robotPosition_->pose.position() - teamBallModel_->absPosition;
  // vector from the target position to the ball
  const Vector2f ballToTarget =
      replacementKeeperAction_->action.pose.position() - teamBallModel_->absPosition;
  // set the replacement keeper position's y coord to something that is not illegal.
  if (ballToRobot.norm() < 0.9f || ballToTarget.norm() < 0.9f)
  {
    const float side = ballToRobot.y() < 0.f ? -1.f : 1.f;
    const float newYCoord =
        teamBallModel_->absPosition.y() +
        (side * std::sqrt(abs(0.9f * 0.9f - ballToRobot.x() * ballToRobot.x())));

    replacementKeeperAction_->action.pose.y() = newYCoord;
  }
}
