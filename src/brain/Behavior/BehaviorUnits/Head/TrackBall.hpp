#pragma once
#include <cmath>

#include "Behavior/Units.hpp"
#include "Tools/Math/Angle.hpp"

ActionCommand::Head trackBall(const DataSet& d, const bool useTeamBall = false)
{
  if (d.teamBallModel.seen && (useTeamBall || !d.ballState.found))
  {
    const Vector2f relBallPos = d.robotPosition.fieldToRobot(d.teamBallModel.position);
    const float relativeBallAngleAbs = std::abs(atan2(relBallPos.y(), relBallPos.x()));
    const float relativeBallDistanceSquared = relBallPos.squaredNorm();

    if (relativeBallAngleAbs < 60.f * TO_RAD || d.teamBallModel.ballType == TeamBallModel::BallType::SELF ||
        (relativeBallAngleAbs < 119.f * TO_RAD && relativeBallDistanceSquared < 2.f * 2.f))
    {
      // the ball is close and can be looked at without breaking the robots neck or was seen by this robot
      // or was seen by this robot
      // or is close
      return ActionCommand::Head::lookAt({relBallPos.x(), relBallPos.y(), d.fieldDimensions.ballDiameter / 2});
    }
    else
    {
      // the robot wouldn't be able to see the ball anyway. Thus simply look around to improve localization
      return lookAround(d);
    }
  }
  else if (d.ballState.found)
  {
    return ActionCommand::Head::lookAt({d.ballState.position.x(), d.ballState.position.y(), d.fieldDimensions.ballDiameter / 2});
  }
  else
  {
    return lookAround(d);
  }
}
