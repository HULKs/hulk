#pragma once

#include "Behavior/Units.hpp"
#include "Modules/NaoProvider.h"
#include "WalkBehindBallAndDribble.hpp"


/**
 * @brief dribble creates an action command for dribbling the ball somewhere
 * @pre The team ball has to be seen.
 * @param d a dataset
 * @param kickPose the relative (!!!) kick pose
 * @param ballDestination the position where the ball should end up
 * @param absolute true iff ballDestination is absolute
 * @param strong whether or not the strong in walk kick should be used
 * @return an action command for dribbling the ball somewhere
 */
ActionCommand dribble(const DataSet& d, const Pose kickPose, const BallUtils::Kickable kickable, const Vector2f& ballDestination, bool absolute = false,
                      bool strong = false)
{
  switch (kickable)
  {
    case BallUtils::Kickable::LEFT:
      if (strong)
      {
        return ActionCommand::walk(Pose(1, 0, 0), WalkMode::DIRECT, Velocity(), InWalkKickType::LEFT_STRONG).combineHead(trackBall(d));
      }
      else
      {
        return ActionCommand::walk(Pose(1, 0, 0), WalkMode::DIRECT, Velocity(), InWalkKickType::LEFT_GENTLE).combineHead(trackBall(d));
      }
    case BallUtils::Kickable::RIGHT:
      if (strong)
      {
        return ActionCommand::walk(Pose(1, 0, 0), WalkMode::DIRECT, Velocity(), InWalkKickType::RIGHT_STRONG).combineHead(trackBall(d));
      }
      else
      {
        return ActionCommand::walk(Pose(1, 0, 0), WalkMode::DIRECT, Velocity(), InWalkKickType::RIGHT_GENTLE).combineHead(trackBall(d));
      }
    case BallUtils::Kickable::NOT:
    default:
      return walkBehindBallAndDribble(d, kickPose).combineHead(trackBall(d));
  }
}
