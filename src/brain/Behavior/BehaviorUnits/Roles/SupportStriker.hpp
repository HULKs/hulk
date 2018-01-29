#pragma once

#include "Behavior/Units.hpp"

ActionCommand support(const DataSet& d)
{
  const Vector2f relBallPosition = d.robotPosition.fieldToRobot(d.teamBallModel.position);
  const float relBallAngle = std::atan2(relBallPosition.y(), relBallPosition.x());
  const Pose supportPose = Pose(d.robotPosition.fieldToRobot(d.supportingPosition.position), relBallAngle);
  return walkToPose(d, supportPose, false).combineHead(trackBall(d, true));
}
