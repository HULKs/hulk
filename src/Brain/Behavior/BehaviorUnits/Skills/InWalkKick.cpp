#include "Brain/Behavior/Units.hpp"

ActionCommand walkToBallAndInWalkKick(const DataSet& d, const Pose& kickPose,
                                      const BallUtils::Kickable kickable,
                                      const InWalkKickType kickType, const Velocity& velocity)
{
  if (kickable == BallUtils::Kickable::LEFT)
  {
    return ActionCommand::walk(Pose(), ActionCommand::Body::WalkMode::DIRECT, Velocity(), kickType,
                               KickFoot::LEFT)
        .combineHead(activeVision(d, VisionMode::BALL_TRACKER));
  }
  if (kickable == BallUtils::Kickable::RIGHT)
  {
    return ActionCommand::walk(Pose(), ActionCommand::Body::WalkMode::DIRECT, Velocity(), kickType,
                               KickFoot::RIGHT)
        .combineHead(activeVision(d, VisionMode::BALL_TRACKER));
  }
  return walkBehindBall(d, kickPose, velocity)
      .combineHead(activeVision(d, VisionMode::BALL_TRACKER));
}
