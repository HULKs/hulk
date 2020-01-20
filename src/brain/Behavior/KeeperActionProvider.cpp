#include <cmath>

#include "Tools/Chronometer.hpp"
#include "Tools/Math/Angle.hpp"
#include "Tools/Math/Hysteresis.hpp"
#include "Tools/Math/Range.hpp"
#include "Tools/PermissionManagement.hpp"

#include "KeeperActionProvider.hpp"

KeeperActionProvider::KeeperActionProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , cycleInfo_(*this)
  , ballState_(*this)
  , fieldDimensions_(*this)
  , gameControllerState_(*this)
  , robotPosition_(*this)
  , teamBallModel_(*this)
  , teamPlayers_(*this)
  , worldState_(*this)
  , keeperAction_(*this)
  , keeperPosition_(Vector2f((-fieldDimensions_->fieldLength * 0.5f) +
                                 fieldDimensions_->fieldPenaltyAreaLength * 0.5f,
                             0.f))
{
}

void KeeperActionProvider::cycle()
{
  Chronometer time(debug(), mount_ + ".cycle_time");
  if (gameControllerState_->gameState != GameState::PLAYING)
  {
    return;
  }

  if (shouldSquat())
  {
    KeeperAction::Action action(KeeperAction::Type::SQUAT);
    keeperAction_->actions.push_back(action);
  }

  if (teamBallModel_->ballType == TeamBallModel::BallType::NONE)
  {
    // fallback to default keeper pose when there is no ball.
    KeeperAction::Action action(KeeperAction::Type::BLOCK_GOAL, Pose(keeperPosition_));
    keeperAction_->actions.push_back(action);
  }
  else
  {
    if (strikerIsInOwnPenaltyArea())
    {
      // if the striker is inside of our own penalty area the keeper position is adjusted to prevent
      // obstructing the striker
      const float minDistanceToBall = 0.75f;
      const float sign = worldState_->ballIsToMyLeft ? -1.f : 1.f;
      const Vector2f waitingPosition(keeperPosition_.x(),
          teamBallModel_->position.y() + sign * minDistanceToBall);
      const Vector2f waitingPositionToBall = teamBallModel_->position - waitingPosition;
      const float orientation = std::atan2(waitingPositionToBall.y(), waitingPositionToBall.x());
      KeeperAction::Action action(KeeperAction::Type::BLOCK_GOAL, Pose(waitingPosition, orientation));
      keeperAction_->actions.push_back(action);
    }
    else
    {
      // go to position between own goal and ball
      const float goalPostPosY = fieldDimensions_->goalInnerWidth / 2.f;
      const float interceptXCoord = keeperPosition_.x();
      // the keeper y-position is obtained by projecting the x-position on the goal-center to ball line
      const float interceptYCoord = Range<float>::clipToGivenRange(
          teamBallModel_->position.y() /
              // abs to avoid sign flip when ball is behind the own goal line, epsilon to prevent division by zero
              (std::abs(teamBallModel_->position.x() + fieldDimensions_->fieldLength / 2) + std::numeric_limits<float>::epsilon()) *
              (interceptXCoord + fieldDimensions_->fieldLength / 2),
          -goalPostPosY, goalPostPosY);
      const Vector2f interceptVec = Vector2f(interceptXCoord, interceptYCoord);
      const float interceptAngle = std::atan2(teamBallModel_->position.y() - keeperPosition_.y(),
          teamBallModel_->position.x() - keeperPosition_.x());
      KeeperAction::Action action(KeeperAction::Type::BLOCK_GOAL, Pose(interceptVec, interceptAngle));
      keeperAction_->actions.push_back(action);
    }
  }

  // find the best action that the keeper is permitted to perform, assuming the actions
  // are sorted
  for (const auto& action : keeperAction_->actions)
  {
    if (action.valid)
    {
      if (PermissionManagement::checkPermission(static_cast<unsigned int>(action.type),
                                                static_cast<unsigned int>(keeperAction_->permission)))
      {
        keeperAction_->action = action;
        break;
      }
    }
  }
}

bool KeeperActionProvider::shouldSquat() const
{
  /// only trust our own balls
  if (!ballState_->found)
  {
    return false;
  }
  const float squatWidth = 0.4f;
  /// ball will come to stop in goal + tolerance
  const bool inGoal = robotPosition_->robotToField(ballState_->destination).x() <
                          (-fieldDimensions_->fieldLength / 2.f + 0.3f) &&
                      std::abs(robotPosition_->robotToField(ballState_->destination).y()) <
                          (fieldDimensions_->goalInnerWidth / 2.f + 0.3f);
  /// ball rolls in direction of goal
  const bool goalDirection =
      robotPosition_->pose.calculateGlobalOrientation(ballState_->velocity).x() < 0;
  /// robot does not look in direction of the goal
  const bool robotLooksForward =
      Angle::angleDiff(robotPosition_->pose.orientation, 0) <= (100 * TO_RAD);

  if (inGoal && goalDirection && std::abs(ballState_->destination.y()) < squatWidth &&
      robotLooksForward)
  {
    return true;
  }
  return false;
}

bool KeeperActionProvider::strikerIsInOwnPenaltyArea() const
{
  for (const auto& teamPlayer : teamPlayers_->players)
  {
    if (teamPlayer.penalized || teamPlayer.currentlyPerformingRole != PlayingRole::STRIKER)
    {
      continue;
    }
    if (teamPlayer.insideOwnPenaltyArea)
    {
      return true;
    }
  }
  return false;
}
