#include "Brain/Behavior/Units.hpp"

ActionCommand walkToBallAndKick(const DataSet& d, const Pose& kickPose,
                                const BallUtils::Kickable kickable, const Vector2f& ballDestination,
                                const bool absolute, const Velocity& velocity,
                                const KickType kickType)
{
  if (d.motionState.bodyMotion == ActionCommand::Body::MotionType::KICK)
  {
    return ActionCommand::stand();
  }

  if (kickable != BallUtils::Kickable::NOT ||
      d.lastRequestedBodyMotionType == ActionCommand::Body::MotionType::KICK)
  {
    const Vector2f relBallDestination =
        absolute ? d.robotPosition.fieldToRobot(ballDestination) : ballDestination;
    return ActionCommand::kick(d.ballState.position, relBallDestination, kickType);
  }
  return walkBehindBall(d, kickPose, velocity)
      .combineHead(activeVision(d, VisionMode::BALL_TRACKER));
}

ActionCommand kickLeft(const DataSet& /*d*/)
{
  return ActionCommand::kick(Vector2f(0.17, 0.05), Vector2f(5, 0.05), KickType::FORWARD);
}

ActionCommand kickRight(const DataSet& /*d*/)
{
  return ActionCommand::kick(Vector2f(0.17, -0.05), Vector2f(5, -0.05), KickType::FORWARD);
}
