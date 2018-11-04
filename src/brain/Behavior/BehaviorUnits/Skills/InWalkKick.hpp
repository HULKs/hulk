#pragma once
#include "Behavior/Units.hpp"

/**
 * @brief walkToBallAndInWalkKick a skill to perform a basic in walk kick. As
 * long as the ball is believed to be not kickable this will fall back to
 * walkBehindBall unsing the kickPose as target.
 * @param d the data set containing some references to important data types (e.g. the world model)
 * @param kickPose the kick pose that is to be approached as long as the ball is noyt kickable
 * @param kickType the type of kick that is to be performed (e.g. forward or turn kick)
 * @param velocity the velocity that is to be used when approaching the ball (full speed if not
 * specified)
 */
ActionCommand walkToBallAndInWalkKick(const DataSet& d, const Pose& kickPose,
                                      const BallUtils::Kickable kickable,
                                      const InWalkKickType kickType = InWalkKickType::FORWARD,
                                      const Velocity& velocity = Velocity())
{
  if (kickable == BallUtils::Kickable::LEFT)
  {
    return ActionCommand::walk(Pose(), WalkMode::DIRECT, Velocity(), kickType, KickFoot::LEFT)
        .combineHead(trackBall(d));
  }
  else if (kickable == BallUtils::Kickable::RIGHT)
  {
    return ActionCommand::walk(Pose(), WalkMode::DIRECT, Velocity(), kickType, KickFoot::RIGHT)
        .combineHead(trackBall(d));
  }
  return walkBehindBall(d, kickPose, velocity).combineHead(trackBall(d));
}
