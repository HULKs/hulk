#pragma once

#include "Behavior/Units.hpp"
#include "Tools/SelectWalkMode.hpp"

ActionCommand bishop(const DataSet& d)
{
  // only use the bishop position if it is valid
  if (d.bishopPosition.valid)
  {
    const Pose relPlayingPose =
        d.robotPosition.fieldToRobot(Pose(d.bishopPosition.position, d.bishopPosition.orientation));

    // select walk mode
    const float distanceThreshold = 1.5f;
    const float angleThreshold = 30 * TO_RAD;
    const WalkMode walkMode = SelectWalkMode::pathOrPathWithOrientation(
        relPlayingPose, distanceThreshold, angleThreshold);

    return walkToPose(d, relPlayingPose, false, walkMode, Velocity(), 5)
        .combineHead(activeVision(d, VisionMode::LOOK_AROUND_BALL));
  }
  else
  {
    Log(LogLevel::WARNING) << "Invalid replacement keeper position";
    return ActionCommand::stand().combineHead(activeVision(d, VisionMode::LOOK_AROUND));
  }
}
