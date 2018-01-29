#pragma once

#include "Behavior/Units.hpp"

#include <cmath>

ActionCommand keeper(const DataSet& d)
{
  switch (d.keeperAction.type)
  {
    case KeeperAction::KICK_BALL_ASAP_AWAY:
    {
      const Vector2f relTeamBall = d.robotPosition.fieldToRobot(d.teamBallModel.position);
      const Vector2f relTarget = d.robotPosition.fieldToRobot(d.keeperAction.target);
      // TODO: move these numbers to somewhere over the rainbow
      int lastSign = 1;
      const float distanceToBall = 0.17f;
      const float angleToBall = 3 * TO_RAD;
      const Pose kickPose = BallUtils::kickPose(relTeamBall, relTarget, distanceToBall, lastSign);
      const BallUtils::Kickable kickable = BallUtils::kickable(kickPose, d.ballState, distanceToBall, angleToBall);
      return walkToBallAndKick(d, kickPose, kickable, relTarget);
    }

    case KeeperAction::GO_CLOSER_TO_CLOSE_BALL:
      return walkToPose(d, d.keeperAction.walkPosition, true).combineHead(trackBall(d));

    case KeeperAction::GO_TO_DEFAULT_POS:
      return walkToPose(d, d.keeperAction.walkPosition, true, WalkMode::PATH, Velocity(), 15).combineHead(lookAround(d));

    case KeeperAction::GENUFLECT:
      return ActionCommand::keeper(MK_TAKE_FRONT);

    case KeeperAction::SEARCH_FOR_BALL:

    default:
      return ActionCommand::stand().combineHead(lookAround(d));
  }
}
