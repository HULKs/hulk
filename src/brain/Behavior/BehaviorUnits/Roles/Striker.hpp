#pragma once

#include "Behavior/Units.hpp"

ActionCommand striker(const DataSet& d)
{
  // It can happen that a striker does not see the ball by itself but due to uncertainty in ball and robot position,
  // the ball is behind the robot even though it thinks it should be in front of it.
  // ballState.found is also checked because it might be that the ball is not in the team ball buffer when walking around the ball.
  // This would lead to ballType becoming TEAM when another robot sees the ball, but ballState.found will probably still be true.
  if (d.teamBallModel.ballType != TeamBallModel::BallType::SELF && !d.ballState.found &&
      (d.teamBallModel.position - d.robotPosition.pose.position).squaredNorm() < 0.5f * 0.5f)
  {
    return rotate(d).combineHead(lookForward(d));
  }
  switch (d.strikerAction.type)
  {
    case StrikerAction::PASS:
      return walkToBallAndKick(d, d.strikerAction.kickPose, d.strikerAction.kickable, d.strikerAction.target, true);
    case StrikerAction::DRIBBLE:
      return dribble(d, d.strikerAction.kickPose, d.strikerAction.kickable, d.strikerAction.target, true);
    case StrikerAction::DRIBBLE_INTO_GOAL:
      return dribble(d, d.strikerAction.kickPose, d.strikerAction.kickable, d.strikerAction.target, true, true);
    case StrikerAction::WAITING_FOR_KEEPER:
      return walkToPose(d, d.strikerAction.kickPose, true);
    case StrikerAction::KICK_INTO_GOAL:
    default:
      return walkToBallAndKick(d, d.strikerAction.kickPose, d.strikerAction.kickable, d.strikerAction.target, true);
  }
}
