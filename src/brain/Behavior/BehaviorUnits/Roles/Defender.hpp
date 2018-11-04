#pragma once

#include "Behavior/Units.hpp"
#include "Tools/SelectWalkMode.hpp"

ActionCommand defender(const DataSet& d)
{
  if (d.defendingPosition.valid)
  {
    const Vector2f relBallPosition = d.robotPosition.fieldToRobot(d.teamBallModel.position);
    const float relBallAngle = std::atan2(relBallPosition.y(), relBallPosition.x());
    const Pose relPlayingPose =
        Pose(d.robotPosition.fieldToRobot(d.defendingPosition.position), relBallAngle);

    // select walk mode
    const float distanceThreshold = 1.5f;
    const float angleThreshold = 30 * TO_RAD;
    const WalkMode walkMode = SelectWalkMode::pathOrPathWithOrientation(
        relPlayingPose, distanceThreshold, angleThreshold);

    return walkToPose(d, relPlayingPose, false, walkMode, Velocity(), 5)
        .combineHead(activeVision(d, VisionMode::LookAroundBall));
  }
  else
  {
    Log(LogLevel::WARNING) << "invalid defending position";
    return ActionCommand::stand().combineHead(trackBall(d));
  }
}
